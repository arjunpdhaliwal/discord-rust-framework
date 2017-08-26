extern crate serde;
extern crate serde_json;

use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Identity {
    pub token: String,
    pub properties: BTreeMap<String, String>,
    pub compress: Option<bool>,
    pub large_threshold: Option<i32>,
}