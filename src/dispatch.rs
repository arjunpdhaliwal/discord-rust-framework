extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub op: i8,
    pub d: serde_json::Value,
    pub s: Option<i16>,
    pub t: Option<String>,
}