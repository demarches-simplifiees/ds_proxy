use std::env;

const DEFAULT_CHUNK_SIZE:usize = 512;

#[derive(Debug)]
pub struct Config {
  pub upstream_base_url: Option<String>,
  pub listen_adress: Option<String>,
  pub listen_port: Option<u16>,
  pub noop: bool,
  pub password: Option<String>,
  pub salt: Option<[u8;32]>,
  pub chunk_size: Option<usize>
}

impl Config {
  pub fn new(salt: [u8; 32], password: &str, chunk_size: usize) -> Config {
    Config{
      salt: Some(salt),
      password: Some(password.to_string()),
      chunk_size: Some(chunk_size),
      ..Config::default()
    }
  }

  fn extract_salt() -> [u8;32] {
    let env = env::var("DS_SALT").unwrap();
    println!("{:?}", env);
    let env_salt = env.as_bytes();
    let mut array = [0; 32];
    let bytes = &env_salt[..array.len()]; // panics if not enough data
    array.copy_from_slice(bytes); 
    array
  }

  pub fn new_from_env() -> Config {
    Config {
      upstream_base_url: env::var("UPSTREAM_URL").ok(),
      password:env::var("DS_PASS").ok(),
      salt: Some(Config::extract_salt()),
      ..Config::default()
    }
  }
}

impl Default for Config {
  fn default() -> Config {
    Config {
      upstream_base_url: None,
      listen_port: None,
      listen_adress: None,
      noop: false,
      password: None,
      salt: None,
      chunk_size: Some(DEFAULT_CHUNK_SIZE),
    }
  }
}
