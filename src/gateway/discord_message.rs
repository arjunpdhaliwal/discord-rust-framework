extern crate serde;
extern crate serde_json;


#[derive(Serialize, Deserialize, Debug)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub author: ::api::User,
    pub content: String,
    pub timestamp: String,
    pub edited_timestamp: Option<String>,
    pub tts: bool,
    pub mention_everyone: bool,
    pub mentions: Vec<serde_json::Value>,
    pub mention_roles: Vec<serde_json::Value>,
    pub attachments: Vec<serde_json::Value>,
    pub embeds: Vec<serde_json::Value>,
    pub reactions: Option<serde_json::Value>,
    pub nonce: Option<String>,
    pub pinned: bool,
    pub webhook_id: Option<String>,
    #[serde(rename = "type")] 
    pub type_id: i8,
}

impl super::MessageData for DiscordMessage { }