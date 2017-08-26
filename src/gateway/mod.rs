extern crate serde;
extern crate serde_json;

pub mod hello;
pub mod ready;
pub mod identity;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message<T> {
    pub op: i8,
    pub d: T,
    pub s: Option<i16>,
    pub t: Option<String>,
}