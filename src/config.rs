#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub resource: String,
    pub server: Server,
    pub room: Room,
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
            room: Room::Default,
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
    Primary,
    Backup,
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Room {
    Default,
    Custom(jid::BareJid),
}

impl Room {
    pub(crate) fn jid(&self, nickname: String) -> jid::FullJid {
        match self {
            Room::Default => jid::FullJid {
                node: Some("NWWS".into()),
                domain: "conference.nwws-oi.weather.gov".into(),
                resource: nickname,
            },
            Room::Custom(jid) => jid::FullJid {
                node: jid.node.clone(),
                domain: jid.domain.clone(),
                resource: nickname,
            },
        }
    }
}

impl Default for Room {
    fn default() -> Self {
        Self::Default
    }
}
