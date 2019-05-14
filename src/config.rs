use std::env;

#[derive(Debug)]
pub struct Config {
  pub upstream_base_url: String,
  pub listen_adress: Option<String>,
  pub listen_port: Option<u16>,
  pub noop: bool,
  pub password: Vec<u8>,
  pub salt: Vec<u8>,
}

impl Config {
  pub fn new() -> Config {
    Config {
      upstream_base_url: env::var("UPSTREAM_URL").unwrap_or(
        "https://storage.gra5.cloud.ovh.net".to_string()
      ),
      listen_port: None,
      listen_adress: None,
      noop: false,
      password: Vec::new(),
      salt: Vec::new(),
    }
  }
}
