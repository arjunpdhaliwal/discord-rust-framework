extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct Hello {
    pub heartbeat_interval: i32,
    pub _trace: Vec<String>,
}

impl super::MessageData for Hello { }