use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Notification<'a> {
    pub hostname: &'a str,
    pub summary:  &'a str,
    pub body:     &'a str,
    pub urgency:  &'a str,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum RequestType {
    Seize,
    Yield,
}
