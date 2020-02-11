use serde::Deserialize;


#[derive(PartialEq)]
pub enum Role {
    Sender,      // this is used to create a notification and send it to the server.
    Server,      // receives notifs from multiple Senders forwards them to ONE unique Notifier
    Notifier,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_ip: String,
    pub incoming_notif_port: String,
    pub yield_port: String,
    pub outgoing_notif_port: String,
}

impl Config {
    pub fn new() -> Result<Config, failure::Error> {

        if let Ok(a) = std::fs::read_to_string("a.txt") {
            //let cfg: Config = toml::from_str(a.as_str()).unwrap();
            //return Ok(cfg);
            return Ok(toml::from_str(a.as_str()).unwrap());
        };

        // if let Ok(a) = std::fs::read_to_string("b.txt") {
        //     return Ok(Config {a});
        // }

        Ok(Config {
            server_ip:           String::from("0"),
            incoming_notif_port: String::from("9111"),
            yield_port:          String::from("9112"),
            outgoing_notif_port: String::from("9112"),
        })
    }
}
