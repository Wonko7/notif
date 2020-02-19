use serde::Deserialize;
use libzmq::auth::CurveCert;

pub fn generate_keys() {
    let cert = CurveCert::new_unique();
    println!("public: \"{}\"", cert.public().as_str());
    println!("secret: \"{}\"", cert.secret().as_str());
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub auth:       libzmq::config::AuthConfig,
    pub sender:     libzmq::config::ClientConfig,
    pub notifier:   libzmq::config::ClientConfig,
    pub router_in:  libzmq::config::ServerConfig,
    pub router_out: libzmq::config::ServerConfig,
}

impl Config {
    pub fn new(file: std::option::Option<&str>) -> Result<Config, failure::Error> {

        if let Some(file) = file {
            let content = std::fs::read_to_string(file)
                .expect(format!("config file {} does not exist", file).as_str());
            return Ok(serde_yaml::from_str(content.as_str())
                .expect(format!("config file {} bad data", file).as_str()));
        };

        if let Some(mut home_config) = dirs::home_dir() {
            home_config.push(".notif");
            if let Ok(content) = std::fs::read_to_string(home_config) {
                return Ok(serde_yaml::from_str(content.as_str())?);
            };
        };

        if let Ok(content) = std::fs::read_to_string("/etc/notif") {
            return Ok(serde_yaml::from_str(content.as_str())?);
        };

        Err(failure::err_msg("no config file"))
    }
}
