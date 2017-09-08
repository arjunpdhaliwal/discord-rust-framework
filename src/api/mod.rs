extern crate serde;
extern crate serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub verified: Option<bool>,
    pub email: Option<String>,
}
