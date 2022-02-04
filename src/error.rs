use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("the configuration is invalid: {0}")]
    Configuration(tokio_xmpp::Error),
    #[error("the credentials were refused: {0}")]
    Credentials(tokio_xmpp::Error),
    #[error("a network error occurred: {0}")]
    Network(tokio_xmpp::Error),
    #[error("an XMPP parse error occurred: {0}")]
    XmppParseError(#[from] xmpp_parsers::Error),
    #[error("the XMPP stream ended")]
    StreamEnded,
}

impl From<tokio_xmpp::Error> for Error {
    fn from(e: tokio_xmpp::Error) -> Self {
        Self::Network(e)
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
