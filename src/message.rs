/// A message received from NWWS-OI.
///
/// See the [NWS Communications Header Policy Document](https://www.weather.gov/tg/awips) for
/// information about how to interpret this data.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Message {
    /// The six character WMO product ID
    pub ttaaii: String,

    /// Four character issuing center
    pub cccc: String,

    /// The six character AWIPS ID, sometimes called AFOS PIL
    pub awips_id: Option<String>,

    /// The time at which this product was issued
    pub issue: chrono::DateTime<chrono::FixedOffset>,

    /// A unique ID for this message
    ///
    /// The id contains two numbers separated by a period. The first number is the UNIX process ID
    /// on the system running the ingest process. The second number is a simple incremented
    /// sequence number for the product. Gaps in the sequence likely indicate message loss.
    pub id: String,

    /// The time at which the message was originally sent by the NWS ingest process to the NWWS-OI
    /// XMPP server, if it differs substantially from the current time.
    ///
    /// See [XEP-0203](https://xmpp.org/extensions/xep-0203.html) for more details.
    pub delay_stamp: Option<chrono::DateTime<chrono::FixedOffset>>,

    /// The LDM sequence number assigned to this product.
    ///
    /// [LDM documentation] states that this value is "[i]gnored by almost everything but existing
    /// due to tradition and history". NWWS OI seems to always prepend such a sequence number to the
    /// message body; this crate parses it out and places it here.
    pub ldm_sequence_number: Option<u32>,

    /// The contents of the message
    pub message: String,
}

impl TryFrom<xmpp_parsers::Element> for Message {
    type Error = ();

    fn try_from(value: xmpp_parsers::Element) -> Result<Self, Self::Error> {
        xmpp_parsers::message::Message::try_from(value)
            .ok()
            .and_then(|msg| Self::try_from(msg).ok())
            .ok_or(())
    }
}

impl TryFrom<xmpp_parsers::message::Message> for Message {
    type Error = xmpp_parsers::message::Message;

    fn try_from(value: xmpp_parsers::message::Message) -> std::result::Result<Self, Self::Error> {
        if value.type_ != xmpp_parsers::message::MessageType::Groupchat {
            return Err(value);
        }

        let delay_stamp = value
            .payloads
            .iter()
            .find(|p| p.is("delay", "urn:xmpp:delay"))
            .and_then(|delay| delay.attr("stamp"))
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok());

        let oi = if let Some(oi) = value.payloads.iter().find(|p| p.is("x", "nwws-oi")) {
            oi
        } else {
            return Err(value);
        };

        let message = oi.text();

        // Some messages have every \n replaced with \n\n
        // Detect and undo that transformation
        let message = if message.matches("\n").count() == message.matches("\n\n").count() * 2 {
            message.replace("\n\n", "\n")
        } else {
            message
        };

        // Fish out the LDM sequence number, if any
        let (ldm_sequence_number, message) = match {
            let mut i = message.splitn(3, '\n');
            (i.next(), i.next().and_then(|s| s.parse().ok()), i.next())
        } {
            (Some(""), Some(ldm_sequence_number), Some(rest)) => {
                (Some(ldm_sequence_number), rest.into())
            }
            _ => (None, message),
        };

        return match (
            oi.attr("awipsid"),
            oi.attr("cccc"),
            oi.attr("id"),
            oi.attr("issue").map(chrono::DateTime::parse_from_rfc3339),
            oi.attr("ttaaii"),
        ) {
            (Some(awipsid), Some(cccc), Some(id), Some(Ok(issue)), Some(ttaaii)) => Ok(Self {
                awips_id: Some(awipsid).filter(|s| s.len() > 0).map(|s| s.into()),
                cccc: cccc.into(),
                id: id.into(),
                issue,
                ttaaii: ttaaii.into(),
                delay_stamp,
                ldm_sequence_number,
                message,
            }),
            _ => Err(value),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    fn msg(xml: &str) -> Result<Message, ()> {
        let element: xmpp_parsers::Element = xml.parse().unwrap();
        let msg: xmpp_parsers::message::Message = element.try_into().unwrap();

        Message::try_from(msg).map_err(|_| ())
    }

    #[test]
    fn parse_banner() {
        assert_eq!(
            msg("<message xmlns=\"jabber:client\" from=\"nwws@conference.nwws-oi.weather.gov\" to=\"w.glynn@nwws-oi.weather.gov/todo\" type=\"groupchat\"><subject>National Weather Wire Service Open Interface</subject><delay xmlns=\"urn:xmpp:delay\" from=\"nwws@conference.nwws-oi.weather.gov\" stamp=\"2015-02-03T20:48:44.222Z\"/></message>"),
            Err(())
        );
    }

    #[test]
    fn parse_terms() {
        assert_eq!(
            msg("<message xmlns=\"jabber:client\" from=\"nwws-oi.weather.gov\" to=\"w.glynn@nwws-oi.weather.gov/uuid/56d00e55-29f5-446a-8e18-0dd6af8e7dcd\"><subject>US Federal Government</subject><body>**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**\n\nThis is a United States Federal Government computer system, which may be\naccessed and used only for official Government business by authorized\npersonnel.  Unauthorized access or use of this computer system may\nsubject violators to criminal, civil, and/or administrative action.\n\nAll information on this computer system may be intercepted, recorded,\nread, copied, and disclosed by and to authorized personnel for official\npurposes, including criminal investigations. Access or use of this\ncomputer system by any person whether authorized or unauthorized,\nCONSTITUTES CONSENT to these terms.\n\n**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**WARNING**</body></message>"),
            Err(())
        );
    }

    #[test]
    fn parse_awips() {
        assert_eq!(
            msg("<message xmlns=\"jabber:client\" to=\"w.glynn@nwws-oi.weather.gov/uuid/25976f21-a846-4e08-8890-d750a95d96a2\" type=\"groupchat\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"><body>KLMK issues RRM valid 2022-02-04T02:54:00Z</body><html xmlns=\"http://jabber.org/protocol/xhtml-im\"><body xmlns=\"http://www.w3.org/1999/xhtml\">KLMK issues RRM valid 2022-02-04T02:54:00Z</body></html><x xmlns=\"nwws-oi\" cccc=\"KLMK\" ttaaii=\"SRUS43\" issue=\"2022-02-04T02:54:00Z\" awipsid=\"RRMLMK\" id=\"14425.25117\"><![CDATA[\n\n987\n\nSRUS43 KLMK 040254\n\nRRMLMK\n\n.ER PRSK2 20220203 Z DC202202040254/DUE/DQG/DH17/HGIFE/DIH1/\n\n.E1 15.4/15.6/15.8/16.1/16.5/17.0/17.6/18.1\n\n.E2 18.6/18.8/18.8/18.9/19.2/19.2/19.3/19.3\n\n.E3 19.2/19.2/19.2/19.1/19.0/19.0/18.8/18.7\n\n.E4 18.6/18.4/18.4/18.4/18.4/18.3/18.2/18.1\n\n.E5 18.1/18.0/17.9/17.9/17.9/17.7/17.7/17.6\n\n.E6 17.5/17.6/17.5/17.4/17.3/17.2/17.2/17.0\n\n]]></x><delay xmlns=\"urn:xmpp:delay\" stamp=\"2022-02-04T02:55:11.810Z\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"/></message>"),
            Ok(Message {
                ttaaii: "SRUS43".into(),
                cccc: "KLMK".into(),
                awips_id: Some(
                    "RRMLMK".into()
                ),
                issue: chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(2, 54, 0), chrono::FixedOffset::east(0)),
                id: "14425.25117".into(),
                delay_stamp: Some(
                    chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(2, 55, 11).with_nanosecond(810_000_000).unwrap(), chrono::FixedOffset::east(0))
                ),
                ldm_sequence_number: Some(987),
                message: "SRUS43 KLMK 040254\nRRMLMK\n.ER PRSK2 20220203 Z DC202202040254/DUE/DQG/DH17/HGIFE/DIH1/\n.E1 15.4/15.6/15.8/16.1/16.5/17.0/17.6/18.1\n.E2 18.6/18.8/18.8/18.9/19.2/19.2/19.3/19.3\n.E3 19.2/19.2/19.2/19.1/19.0/19.0/18.8/18.7\n.E4 18.6/18.4/18.4/18.4/18.4/18.3/18.2/18.1\n.E5 18.1/18.0/17.9/17.9/17.9/17.7/17.7/17.6\n.E6 17.5/17.6/17.5/17.4/17.3/17.2/17.2/17.0\n".into(),
            })
        );

        assert_eq!(
            msg("<message xmlns=\"jabber:client\" to=\"w.glynn@nwws-oi.weather.gov/uuid/851c737e-ead3-460d-b0a6-6749602fccd9\" type=\"groupchat\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"><body>PAJK issues RR3 valid 2022-02-04T02:11:00Z</body><html xmlns=\"http://jabber.org/protocol/xhtml-im\"><body xmlns=\"http://www.w3.org/1999/xhtml\">PAJK issues RR3 valid 2022-02-04T02:11:00Z</body></html><x xmlns=\"nwws-oi\" cccc=\"PAJK\" ttaaii=\"SRAK57\" issue=\"2022-02-04T02:11:00Z\" awipsid=\"RR3AJK\" id=\"14425.24041\"><![CDATA[\n\n876\n\nSRAK57 PAJK 040211\n\nRR3AJK\n\nSRAK57 PAJK 040210\n\n\n\n.A NDIA2 220204 Z DH0202/TA 26/TD 27/UD 0/US 0/UG 0/UP 0/PA 29.57\n\n]]></x></message>"),
            Ok(Message {
                ttaaii: "SRAK57".into(),
                cccc: "PAJK".into(),
                awips_id: Some("RR3AJK".into()),
                issue: chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(2, 11, 0), chrono::FixedOffset::east(0)),
                id: "14425.24041".into(),
                delay_stamp: None,
                ldm_sequence_number: Some(876),
                message: "SRAK57 PAJK 040211\nRR3AJK\nSRAK57 PAJK 040210\n\n.A NDIA2 220204 Z DH0202/TA 26/TD 27/UD 0/US 0/UG 0/UP 0/PA 29.57\n".into(),
            }));

        assert_eq!(
            msg("<message xmlns=\"jabber:client\" to=\"w.glynn@nwws-oi.weather.gov/uuid/851c737e-ead3-460d-b0a6-6749602fccd9\" type=\"groupchat\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"><body>KKCI issues CFP valid 2022-02-04T02:00:00Z</body><html xmlns=\"http://jabber.org/protocol/xhtml-im\"><body xmlns=\"http://www.w3.org/1999/xhtml\">KKCI issues CFP valid 2022-02-04T02:00:00Z</body></html><x xmlns=\"nwws-oi\" cccc=\"KKCI\" ttaaii=\"FAUS29\" issue=\"2022-02-04T02:00:00Z\" awipsid=\"CFP03\" id=\"14425.22838\"><![CDATA[\n\n631\n\nFAUS29 KKCI 040200\n\nCFP03 \n\nCCFP 20220204_0200 20220204_0800\n\nCANADA OFF\n\n]]></x></message>"),
            Ok(Message{
                ttaaii: "FAUS29".to_string(),
                cccc: "KKCI".to_string(),
                awips_id: Some("CFP03".into()),
                issue: chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(2, 0, 0), chrono::FixedOffset::east(0)),
                id: "14425.22838".into(),
                delay_stamp: None,
                ldm_sequence_number: Some(631),
                message: "FAUS29 KKCI 040200\nCFP03 \nCCFP 20220204_0200 20220204_0800\nCANADA OFF\n".into()
            }));
    }

    #[test]
    fn parse_test() {
        assert_eq!(
            msg("<message xmlns=\"jabber:client\" to=\"w.glynn@nwws-oi.weather.gov/uuid/851c737e-ead3-460d-b0a6-6749602fccd9\" type=\"groupchat\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"><body>PHEB issues  valid 2022-02-04T01:23:00Z</body><html xmlns=\"http://jabber.org/protocol/xhtml-im\"><body xmlns=\"http://www.w3.org/1999/xhtml\">PHEB issues  valid 2022-02-04T01:23:00Z</body></html><x xmlns=\"nwws-oi\" cccc=\"PHEB\" ttaaii=\"NTXX98\" issue=\"2022-02-04T01:23:00Z\" awipsid=\"\" id=\"14425.22800\"><![CDATA[\n\n593\n\nNTXX98 PHEB 040123\n\nPTWC REDUNDANT-SIDE TEST FROM IRC\n\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\n\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\n\n]]></x></message>"),
            Ok(Message {
                ttaaii: "NTXX98".into(),
                cccc: "PHEB".into(),
                awips_id: None,
                issue: chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(1, 23, 0), chrono::FixedOffset::east(0)),
                id: "14425.22800".into(),
                delay_stamp: None,
                ldm_sequence_number: Some(593),
                message: "NTXX98 PHEB 040123\nPTWC REDUNDANT-SIDE TEST FROM IRC\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\n".into(),
            })
        );
    }
}
