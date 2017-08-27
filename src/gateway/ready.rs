extern crate serde;
extern crate serde_json;


#[derive(Serialize, Deserialize, Debug)]
pub struct Ready {
    pub v: i8,
    pub user: serde_json::Value,
    pub private_channels: Vec<serde_json::Value>,
    pub guilds: Vec<serde_json::Value>,
    pub session_id: String,
    pub _trace: Vec<String>,
}

impl super::MessageData for Ready { }