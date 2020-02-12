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
    pub fn new() -> Result<Config, failure::Error> {
        let mut home_config = dirs::home_dir().unwrap(); // .push(".notifier"); <- this does not work?
        home_config.push(".notifier");
        if let Ok(a) = std::fs::read_to_string(home_config) {
            return Ok(toml::from_str(a.as_str()).unwrap());
        };

        if let Ok(a) = std::fs::read_to_string("/etc/notifier") {
            return Ok(toml::from_str(a.as_str()).unwrap());
        };

        Ok(Config {
            server_ip:           String::from("0"),
            incoming_notif_port: 9691,
            notifier_seize_port: 9692,
            outgoing_notif_port: 9693,
        })
    }
}
