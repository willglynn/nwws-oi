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

        match (
            oi.attr("awipsid"),
            oi.attr("cccc"),
            oi.attr("id"),
            oi.attr("issue").map(chrono::DateTime::parse_from_rfc3339),
            oi.attr("ttaaii"),
        ) {
            (Some(awipsid), Some(cccc), Some(id), Some(Ok(issue)), Some(ttaaii)) => {
                return Ok(Self {
                    awips_id: Some(awipsid).filter(|s| s.len() > 0).map(|s| s.into()),
                    cccc: cccc.into(),
                    id: id.into(),
                    issue,
                    ttaaii: ttaaii.into(),
                    delay_stamp,
                    message,
                });
            }
            _ => return Err(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            msg("<message xmlns=\"jabber:client\" to=\"w.glynn@nwws-oi.weather.gov/uuid/851c737e-ead3-460d-b0a6-6749602fccd9\" type=\"groupchat\" from=\"nwws@conference.nwws-oi.weather.gov/nwws-oi\"><body>PAJK issues RR3 valid 2022-02-04T02:11:00Z</body><html xmlns=\"http://jabber.org/protocol/xhtml-im\"><body xmlns=\"http://www.w3.org/1999/xhtml\">PAJK issues RR3 valid 2022-02-04T02:11:00Z</body></html><x xmlns=\"nwws-oi\" cccc=\"PAJK\" ttaaii=\"SRAK57\" issue=\"2022-02-04T02:11:00Z\" awipsid=\"RR3AJK\" id=\"14425.24041\"><![CDATA[\n\n876\n\nSRAK57 PAJK 040211\n\nRR3AJK\n\nSRAK57 PAJK 040210\n\n\n\n.A NDIA2 220204 Z DH0202/TA 26/TD 27/UD 0/US 0/UG 0/UP 0/PA 29.57\n\n]]></x></message>"),
            Ok(Message {
                ttaaii: "SRAK57".into(),
                cccc: "PAJK".into(),
                awips_id: Some("RR3AJK".into()),
                issue: chrono::DateTime::from_utc(chrono::NaiveDate::from_ymd(2022, 2, 4).and_hms(2, 11, 0), chrono::FixedOffset::east(0)),
                id: "14425.24041".into(),
                delay_stamp: None,
                message: "\n876\nSRAK57 PAJK 040211\nRR3AJK\nSRAK57 PAJK 040210\n\n.A NDIA2 220204 Z DH0202/TA 26/TD 27/UD 0/US 0/UG 0/UP 0/PA 29.57\n".into(),
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
                message: "\n631\nFAUS29 KKCI 040200\nCFP03 \nCCFP 20220204_0200 20220204_0800\nCANADA OFF\n".into()
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
                message: "\n593\nNTXX98 PHEB 040123\nPTWC REDUNDANT-SIDE TEST FROM IRC\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\nRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZRZ\n".into(),
            })
        );
    }
}
