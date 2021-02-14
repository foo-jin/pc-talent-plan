use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Ping {
    pub msg: Option<String>,
}

impl Ping {
    pub fn empty() -> Self {
        Ping { msg: None }
    }

    pub fn with_msg(msg: impl Into<String>) -> Self {
        Ping {
            msg: Some(msg.into()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum PingResponse {
    #[serde(rename = "PONG")]
    Pong,
    Echo(String),
}
