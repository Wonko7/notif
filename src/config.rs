use serde::Deserialize;
use libzmq::{auth::{CurvePublicKey, CurveSecretKey}, Heartbeat, Period, TcpAddr};

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


#[derive(Deserialize, Debug)]
pub struct SrvConfig {
    pub incoming: TcpAddr,
    pub outgoing: TcpAddr,
    pub secret:   CurveSecretKey,
    pub auth:     libzmq::config::AuthConfig,
}

#[derive(Deserialize, Debug)]
pub struct SrvToConnect {
    pub incoming: TcpAddr,
    pub outgoing: TcpAddr,
    pub public:   CurvePublicKey,
}

#[derive(Deserialize, Debug)]
pub struct CliConfig {
    pub server: SrvToConnect,
    pub public: CurvePublicKey,
    pub secret: CurveSecretKey,
}

#[derive(Deserialize, Debug)]
pub struct RecvConfig {
    pub send_hwm: i32,
    pub send_timeout: Period,
}

#[derive(Deserialize, Debug)]
pub struct SendConfig {
    pub send_hwm: i32,
    pub send_timeout: Period,
}

#[derive(Deserialize, Debug)]
pub struct FileConfig {
    pub as_server: SrvConfig,
    pub as_client: CliConfig,
    pub heartbeat: Option<Heartbeat>,
    pub send_config: Option<SendConfig>,
    pub recv_config: Option<RecvConfig>,
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


pub fn read_conf() ->  Result<FileConfig, failure::Error> {
    let content = std::fs::read_to_string("misc/notif-example-conf.yaml")?;
    let mut fc: FileConfig = serde_yaml::from_str(content.as_str())?;
    let soc_conf = "tcp://192.168.0.1:9999";

    Ok(fc)
}
