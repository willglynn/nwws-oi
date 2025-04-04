/// Settings used to connect to the NWWS OI.
///
/// # Example
///
/// ```rust
/// let config = nwws_oi::Config::from(("user", "pass"));
///
/// assert_eq!(config, nwws_oi::Config {
///   username: "user".to_string(),
///   password: "pass".to_string(),
///   resource: config.resource.clone(),    // assigned randomly
///   server: nwws_oi::Server::Primary,
///   channel: nwws_oi::Channel::Default,
/// });
///
/// assert!(config.resource.starts_with("uuid/"));
/// ```
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Config {
    /// The username assigned by NWS.
    ///
    /// [Sign up](https://www.weather.gov/nwws/nwws_oi_request) to get your own.
    pub username: String,
    /// The password assigned by NWS.
    pub password: String,
    /// The XMPP resource used for this connection.
    ///
    /// The resource must be unique for your username. If multiple connections attempt to use the
    /// same resource, they will interfere with each other.
    pub resource: String,
    /// The destination server.
    pub server: Server,
    /// The MUC room which contains NWWS OI messages.
    pub channel: Channel,
}

impl Config {
    pub(crate) fn jid(&self) -> String {
        format!(
            "{}@{}/{}",
            &self.username,
            &self.server.hostname(),
            &self.resource,
        )
    }
}

impl From<(String, String)> for Config {
    fn from((username, password): (String, String)) -> Self {
        Self {
            username,
            password,
            resource: format!("uuid/{}", uuid::Uuid::new_v4()),
            server: Server::Primary,
            channel: Channel::Default,
        }
    }
}

impl From<(&str, &str)> for Config {
    fn from((username, password): (&str, &str)) -> Self {
        (username.to_string(), password.to_string()).into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Server {
    /// The primary NWWS OI server.
    Primary,
    /// The backup NWWS OI server.
    Backup,
    /// A custom hostname.
    Custom(String),
}

impl Server {
    pub(crate) fn hostname(&self) -> &str {
        match self {
            Server::Primary => "nwws-oi.weather.gov",
            Server::Backup => "nwws-oi-md.weather.gov",
            Server::Custom(name) => name,
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::Primary
    }
}

/// An XMPP MUC chat room used for disseminating NWWS messages.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Channel {
    Default,
    Custom(jid::BareJid),
}

impl Channel {
    pub(crate) fn jid(&self, nickname: &jid::ResourceRef) -> jid::FullJid {
        match self {
            Channel::Default => jid::FullJid::from_parts(
                Some(&jid::NodePart::new("NWWS").unwrap()),
                &jid::DomainPart::new("conference.nwws-oi.weather.gov").unwrap(),
                nickname,
            ),
            Channel::Custom(jid) => jid::FullJid::from_parts(jid.node(), jid.domain(), nickname),
        }
    }
}

impl Default for Channel {
    fn default() -> Self {
        Self::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn server() {
        assert_eq!(Server::Primary, Default::default());

        // https://www.weather.gov/nwws/configuration
        assert_eq!(Server::Primary.hostname(), "nwws-oi.weather.gov");
        assert_eq!(Server::Backup.hostname(), "nwws-oi-md.weather.gov");
        assert_eq!(Server::Custom("foo".into()).hostname(), "foo");
    }

    #[test]
    fn channel() {
        assert_eq!(Channel::Default, Default::default());

        let foo = jid::ResourcePart::new("foo").unwrap();

        assert_eq!(
            Channel::Default.jid(&foo),
            "NWWS@conference.nwws-oi.weather.gov/foo"
                .parse::<jid::FullJid>()
                .unwrap()
        );

        assert_eq!(
            Channel::Custom(jid::BareJid::from_str("bar@baz").unwrap()).jid(&foo),
            "bar@baz/foo".parse::<jid::FullJid>().unwrap()
        );
    }
}
