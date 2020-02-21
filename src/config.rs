use failure::{Error, err_msg};
use std::{fs::File, io::Write};

use serde::{Deserialize, Serialize};
use libzmq::{auth::{CurveCert, CurvePublicKey, CurveSecretKey}, TcpAddr, config::AuthConfig};

#[derive(Deserialize, Serialize, Debug)]
pub struct SrvConfig {
    pub incoming: TcpAddr,
    pub outgoing: TcpAddr,
    pub secret:   CurveSecretKey,
    pub auth:     AuthConfig,
}
impl SrvConfig {
    fn new(incoming: &TcpAddr, outgoing: &TcpAddr, secret: &CurveSecretKey, auth: &AuthConfig) -> SrvConfig {
        SrvConfig { // not pretty, but only used to make topo config files.
            incoming: incoming.clone(),
            outgoing: outgoing.clone(),
            secret:   secret.clone(),
            auth:     auth.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SrvToConnect {
    pub incoming: TcpAddr,
    pub outgoing: TcpAddr,
    pub public:   CurvePublicKey,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CliConfig {
    pub server: SrvToConnect,
    pub cert:   CurveCert,
}
impl CliConfig {
    fn new(incoming: &TcpAddr, outgoing: &TcpAddr, public: &CurvePublicKey, cert: &CurveCert) -> CliConfig {
        CliConfig { // not pretty, but only used to make topo config files.
            server: SrvToConnect {
                incoming: incoming.clone(),
                outgoing: outgoing.clone(),
                public:   public.clone(),
            },
            cert: cert.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_server: Option<SrvConfig>,
    pub as_client: CliConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
}

impl Config {
    pub fn new(file: std::option::Option<&str>) -> Result<Config, Error> {

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

        Err(err_msg("no config file"))
    }

    fn as_client(as_client: CliConfig) -> Config {
        Config {
            as_client,
            as_server: None,
            verbose: None,
        }
    }

    fn as_server(as_client: CliConfig, as_server: SrvConfig) -> Config {
        Config {
            as_client,
            as_server: Some(as_server),
            verbose: None,
        }
    }
}

pub fn generate_keys() {
    let cert = CurveCert::new_unique();
    println!("public: \"{}\"", cert.public().as_str());
    println!("secret: \"{}\"", cert.secret().as_str());
}

pub fn generate_topo(incoming: &TcpAddr, outgoing: &TcpAddr, nb_clients: u32) -> Result<(), Error> {
    let server_cert  = CurveCert::new_unique();
    let mut registry = Vec::new();

    for i in 0..nb_clients {
        println!("{}", i);
        let cli_cert   = CurveCert::new_unique();
        let as_client  = CliConfig::new(&incoming, &outgoing, server_cert.public(), &cli_cert);
        let cli_config = Config::as_client(as_client);

        let mut client_file = File::create(format!("client-{}.notif", i))?;
        write!(client_file, "{}", serde_yaml::to_string(&cli_config)?)?;

        registry.push(cli_cert.public().clone());
    }

    let mut srv_auth = AuthConfig::new();
    let cli_cert     = CurveCert::new_unique();
    let as_client    = CliConfig::new(&incoming, &outgoing, server_cert.public(), &cli_cert);
    registry.push(cli_cert.public().clone());
    srv_auth.set_curve_registry(Some(&registry));
    let as_server    = SrvConfig::new(&incoming, &outgoing, server_cert.secret(), &srv_auth);
    let srv_config   = Config::as_server(as_client, as_server);

    let mut srv_file = File::create("server.notif")?;
    write!(srv_file, "{}", serde_yaml::to_string(&srv_config)?)?;

    Ok(())
}
