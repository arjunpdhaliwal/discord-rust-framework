extern crate serde;
extern crate serde_json;

pub mod hello;
pub mod ready;
pub mod identity;

pub trait MessageData { }

#[derive(Serialize, Debug)]
pub struct MessageBody<T: MessageData> {
    pub op: i8,
    pub d: T,
    pub s: Option<i16>,
    pub t: Option<String>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Serialize, Debug)]
pub struct ClientMessage {
    pub body: String,
    pub priority: i32,
}   

#[derive(Deserialize, Debug)]
pub struct ServerMessage {
    pub op: i8,
    pub d: serde_json::Value,
    pub s: Option<i16>,
    pub t: Option<String>,
}