extern crate serde;
extern crate serde_json;

use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Data {
    pub token: String,
    pub properties: BTreeMap<String, String>,
    pub compress: Option<bool>,
    pub large_threshold: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub op: i32,
    pub d: Data,
}
