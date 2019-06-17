use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::*;
use std::env;
use actix_web::http::Uri;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use super::args;

pub type DsKey = Key;

// match nginx default (proxy_buffer_size in ngx_stream_proxy_module)
pub const DEFAULT_CHUNK_SIZE: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct Config {
    pub upstream_base_url: Option<String>,
    pub noop: bool,
    pub password: Option<String>,
    pub salt: Option<String>,
    pub chunk_size: Option<usize>,
}

impl Config {
    pub fn new(salt: &str, password: &str, chunk_size: usize) -> Config {
        Config {
            salt: Some(salt.to_string()),
            password: Some(password.to_string()),
            chunk_size: Some(chunk_size),
            ..Config::default()
        }
    }

    fn new_from_env() -> Config {
        let chunk_size:usize = match env::var("DS_CHUNK_SIZE") {
            Ok(chunk_str) => chunk_str.parse::<usize>().unwrap_or(DEFAULT_CHUNK_SIZE),
            _ => DEFAULT_CHUNK_SIZE
        };

        Config {
            upstream_base_url: env::var("UPSTREAM_URL").ok(),
            salt: env::var("DS_SALT").ok(),
            chunk_size: Some(chunk_size),
            ..Config::default()
        }
    }

    pub fn create_key(self) -> Result<Key, &'static str> {
        match (self.password, self.salt) {
            (Some(password), Some(input_salt)) => {
                if let Some(salt) = Salt::from_slice(&input_salt.as_bytes()[..]) {
                    let mut raw_key = [0u8; KEYBYTES];

                    pwhash::derive_key(
                        &mut raw_key,
                        password.as_bytes(),
                        &salt,
                        pwhash::OPSLIMIT_INTERACTIVE,
                        pwhash::MEMLIMIT_INTERACTIVE,
                    )
                    .unwrap();

                    Ok(Key(raw_key))
                } else {
                    Err("Unable to derive a key from the salt")
                }
            }
            _ => Err("Password or salt is missing. Impossible to derive a key"),
        }
    }

    pub fn create_url(&self, uri: &Uri) -> String {
        format!("{}{}", self.upstream_base_url.clone().unwrap(), uri)
    }

    pub fn create_config(args: &args::Args) -> Config {
        Config{
            password: Some(read_password(args.arg_password_file.clone().unwrap())),
            noop: args.flag_noop,
            ..Config::new_from_env()
        }
    }
}

fn read_password(path: String) -> String {
    let file = File::open(path).unwrap();
    let reader = io::BufReader::new(file);
    reader.lines().nth(0).unwrap().unwrap()
}


impl Default for Config {
    fn default() -> Config {
        Config {
            upstream_base_url: None,
            noop: false,
            password: None,
            salt: None,
            chunk_size: Some(DEFAULT_CHUNK_SIZE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_creation() {
        let passwd = "Correct Horse Battery Staple";
        let salt = "abcdefghabcdefghabcdefghabcdefgh";
        let config_ok = Config::new(&salt.to_string(), passwd, 512);
        let config_no_salt = Config {
            password: Some(passwd.to_string()),
            ..Config::default()
        };
        let config_no_password = Config {
            salt: Some(salt.to_string()),
            ..Config::default()
        };

        assert_eq!(true, config_ok.create_key().is_ok());
        assert_eq!(true, config_no_salt.create_key().is_err());
        assert_eq!(true, config_no_password.create_key().is_err());
    }
}
