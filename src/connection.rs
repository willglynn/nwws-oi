use crate::*;
use futures::{StreamExt, TryStreamExt};
use log::{debug, error, info, log_enabled, trace, warn, Level};

/// A connection to NWWS-OI.
///
/// `Connection` is 1:1 with an underlying XMPP connection. Failures are generally unrecoverable.
/// Most users will prefer to use [`Stream`](struct.Stream.html) instead.
pub struct Connection {
    client: tokio_xmpp::SimpleClient,
    leave_message: xmpp_parsers::Element,
}

impl Connection {
    /// Connect to NWWS-OI.
    ///
    /// `new()` returns `Ok(Connection)` once the XMPP connection is established, authenticated, and
    /// joined to the NWWS MUC. If any of these steps fail, it returns `Err(Error)`.
    pub async fn new<C: Into<Config>>(config: C) -> Result<Self> {
        let config = config.into();
        let jid = config.jid();
        let Config {
            username,
            resource,
            password,
            channel,
            ..
        } = config;
        let nickname = format!("{}/{}", username, resource);

        // Connect
        info!("connecting to {}", &config.server.hostname());
        let mut client = tokio_xmpp::SimpleClient::new(&jid, password)
            .await
            .map_err(|e| {
                error!("connection failed: {}", e);
                match e {
                    tokio_xmpp::Error::JidParse(_) => Error::Configuration(e),
                    tokio_xmpp::Error::Auth(_) => Error::Credentials(e),
                    _ => Error::Network(e),
                }
            })?;
        let jid = client.bound_jid().clone();
        debug!("connected as {}", &jid);

        // Build the message to join the MUC
        let channel_jid = channel.jid(nickname);
        let join_message =
            xmpp_parsers::presence::Presence::new(xmpp_parsers::presence::Type::None)
                .with_from(jid.clone())
                .with_to(channel_jid.clone())
                .with_payloads(vec![xmpp_parsers::muc::Muc {
                    password: None,
                    history: Some(xmpp_parsers::muc::muc::History {
                        maxchars: None,
                        maxstanzas: None,
                        seconds: Some(300),
                        since: None,
                    }),
                }
                .into()]);
        debug!("joining channel {}", &channel_jid);

        // Build the message to leave the MUC
        //   https://xmpp.org/extensions/xep-0045.html#bizrules-presence § 17.3.2
        let leave_message =
            xmpp_parsers::presence::Presence::new(xmpp_parsers::presence::Type::Unavailable)
                .with_from(join_message.from.as_ref().unwrap().clone())
                .with_to(join_message.to.as_ref().unwrap().clone())
                .with_payloads(vec![xmpp_parsers::muc::Muc {
                    password: None,
                    history: None,
                }
                .into()])
                .into();

        // Join the MUC, and wait for the join to complete
        client.send_stanza(join_message).await?;
        'wait_for_join: loop {
            let item = client.try_next().await?.ok_or(Error::StreamEnded)?;

            if let Ok(presence) = xmpp_parsers::presence::Presence::try_from(item.clone()) {
                for payload in presence.payloads {
                    if let Ok(muc_user) = xmpp_parsers::muc::MucUser::try_from(payload) {
                        if muc_user
                            .status
                            .iter()
                            .any(|s| s == &xmpp_parsers::muc::user::Status::SelfPresence)
                        {
                            break 'wait_for_join;
                        }
                    }
                }
            }
        }

        info!(
            "connected to NWWS-OI {} and joined channel {}",
            &jid, &channel_jid
        );

        Ok(Self {
            client,
            leave_message,
        })
    }

    /// Terminate the connection as gracefully as possible.
    pub async fn end(self) {
        let mut client = self.client;

        // Attempt to leave the room, ignoring errors
        client.send_stanza(self.leave_message).await.ok();

        // Attempt to end the stream, ignoring errors
        client.end().await.ok();

        // Dropping client closes the connection
    }

    /// Receive the next message from NWWS-OI.
    pub async fn next_message(&mut self) -> Result<Message> {
        loop {
            let element = self.client.next().await.ok_or(Error::StreamEnded)??;

            if log_enabled!(Level::Trace) {
                let mut xml = Vec::new();
                element
                    .write_to(&mut std::io::Cursor::new(&mut xml))
                    .expect("encode");
                let xml = String::from_utf8(xml).expect("UTF-8");
                trace!("received: {}", xml);
            }

            if element.is("message", "jabber:client") {
                if let Ok(msg) = Message::try_from(element.clone()) {
                    return Ok(msg);
                }
            } else if element.is("iq", "jabber:client") {
                let iq = xmpp_parsers::iq::Iq::try_from(element)?;
                self.handle_iq(iq).await?;
            } else if element.is("presence", "jabber:client") {
                trace!("presence message: {:?}", element);
            } else {
                warn!("unhandled message: {:?}", element);
            }
        }
    }

    async fn handle_iq(&mut self, iq: xmpp_parsers::iq::Iq) -> Result<()> {
        // We may need to respond to this IQ:
        //
        //     If an entity receives an IQ stanza of type "get" or "set" containing a child element
        //     qualified by a namespace it does not understand, the entity SHOULD return an IQ
        //     stanza of type "error" with an error condition of <service-unavailable/>.
        match &iq.payload {
            xmpp_parsers::iq::IqType::Get(_) | xmpp_parsers::iq::IqType::Set(_) => {
                debug!(
                    "responding to IQ{} with service-unavailable",
                    iq.from
                        .as_ref()
                        .map(|j| format!(" from {}", j))
                        .unwrap_or_default()
                );

                let stanza = xmpp_parsers::iq::Iq {
                    from: iq.to,
                    to: iq.from,
                    id: iq.id,
                    payload: xmpp_parsers::iq::IqType::Error(
                        xmpp_parsers::stanza_error::StanzaError {
                            type_: xmpp_parsers::stanza_error::ErrorType::Cancel,
                            by: None,
                            defined_condition:
                                xmpp_parsers::stanza_error::DefinedCondition::ServiceUnavailable,
                            texts: Default::default(),
                            other: None,
                        },
                    ),
                };

                self.client.send_stanza(stanza).await?;
            }
            _ => {}
        };

        Ok(())
    }
}
