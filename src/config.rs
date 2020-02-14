extern crate serde;
use serde::Deserialize;


// #[derive(PartialEq)]
// pub enum Role {
//     Sender,      // this is used to create a notification and send it to the server.
//     Server,      // receives notifs from multiple Senders forwards them to ONE unique Notifier
//     Notifier,
// }

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_ip:           String,
    pub incoming_notif_port: u16,
    pub notifier_seize_port: u16,
    pub outgoing_notif_port: u16,
}

impl Config {
    pub fn new(file: std::option::Option<&str>) -> Result<Config, failure::Error> {

        if let Some(file) = file {
            let content = std::fs::read_to_string(file).expect(format!("config file {} does not exist", file).as_str());
            return Ok(toml::from_str(content.as_str()).unwrap());
        }

        let mut home_config = dirs::home_dir().unwrap(); // .push(".notifier"); <- this does not work?
        home_config.push(".notif");
        if let Ok(content) = std::fs::read_to_string(home_config) {
            return Ok(toml::from_str(content.as_str()).unwrap());
        };

        if let Ok(content) = std::fs::read_to_string("/etc/notif") {
            return Ok(toml::from_str(content.as_str()).unwrap());
        };

        Ok(Config {
            server_ip:           String::from("0"),
            incoming_notif_port: 9691,
            notifier_seize_port: 9692,
            outgoing_notif_port: 9693,
        })
    }
}
