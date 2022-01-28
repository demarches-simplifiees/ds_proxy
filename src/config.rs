use super::args;
use actix_web::HttpRequest;
use sodiumoxide::crypto::pwhash;
use sodiumoxide::crypto::pwhash::argon2i13::{pwhash_verify, HashedPassword};
use sodiumoxide::crypto::pwhash::scryptsalsa208sha256::Salt;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::*;
use std::env;
use std::net::{SocketAddr, ToSocketAddrs};
use url::Url;

pub type DsKey = Key;

// match nginx default (proxy_buffer_size in ngx_stream_proxy_module)
pub const DEFAULT_CHUNK_SIZE: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct Config {
    pub upstream_base_url: Option<String>,
    pub noop: bool,
    pub key: DsKey,
    pub chunk_size: usize,
    pub max_connections: usize,
    pub input_file: Option<String>,
    pub output_file: Option<String>,
    pub address: Option<SocketAddr>,
}

impl Config {
    pub fn create_config(args: &args::Args) -> Config {
        let password = match &args.flag_password_file {
            Some(password_file) => read_file_content(password_file),
            None => env::var("DS_PASSWORD")
                .expect("Missing password, use DS_PASSWORD env or --password-file cli argument"),
        };

        let password_hash = match &args.flag_hash_file {
            Some(hash_file) => read_file_content(hash_file),
            None => env::var("DS_PASSWORD_HASH")
                .expect("Missing hash, use DS_PASSWORD_HASH env or --hash-file cli argument"),
        };

        ensure_valid_password(&password, &password_hash);

        let salt = match &args.flag_salt {
            Some(salt) => salt.to_string(),
            None => {
                env::var("DS_SALT").expect("Missing salt, use DS_SALT env or --salt cli argument")
            }
        };

        let chunk_size = match &args.flag_chunk_size {
            Some(chunk_size) => *chunk_size,
            None => match env::var("DS_CHUNK_SIZE") {
                Ok(chunk_str) => chunk_str.parse::<usize>().unwrap_or(DEFAULT_CHUNK_SIZE),
                _ => DEFAULT_CHUNK_SIZE,
            },
        };

        let upstream_base_url = if args.cmd_proxy {
            match &args.flag_upstream_url {
                Some(upstream_url) => Some(upstream_url.to_string()),
                None => Some(env::var("DS_UPSTREAM_URL").expect(
                    "Missing upstream_url, use DS_UPSTREAM_URL env or --upstream-url cli argument",
                )),
            }
        } else {
            None
        };

        let address = if args.cmd_proxy {
            match &args.flag_address {
                Some(address) => match address.to_socket_addrs() {
                    Ok(mut sockets) => Some(sockets.next().unwrap()),
                    _ => panic!("Unable to parse the address"),
                },
                None => match (env::var("DS_ADDRESS")
                    .expect("Missing address, use DS_ADDRESS env or --address cli argument"))
                .to_socket_addrs()
                {
                    Ok(mut sockets) => Some(sockets.next().unwrap()),
                    _ => panic!("Unable to parse the address"),
                },
            }
        } else {
            None
        };

        Config {
            key: create_key(salt, password).unwrap(),
            chunk_size,
            upstream_base_url,
            noop: args.flag_noop,
            input_file: args.arg_input_file.clone(),
            output_file: args.arg_output_file.clone(),
            address,
            max_connections: args.flag_max_connections.unwrap_or(25_000),
        }
    }

    pub fn create_upstream_url(&self, req: &HttpRequest) -> String {
        let base = Url::parse(self.upstream_base_url.as_ref().unwrap()).unwrap();
        let mut url = base.join(&req.match_info()["name"]).unwrap();

        if !req.query_string().is_empty() {
            url.set_query(Some(req.query_string()));
        }

        url.to_string()
    }
}

fn read_file_content(path_string: &str) -> String {
    match std::fs::read(path_string) {
        Err(why) => panic!("couldn't open {}: {}", path_string, why),
        Ok(file) => String::from_utf8(file).unwrap(),
    }
}

fn ensure_valid_password(password: &str, hash: &str) {
    let hash = HashedPassword::from_slice(hash.as_bytes());

    if !pwhash_verify(&hash.unwrap(), password.trim_end().as_bytes()) {
        panic!("Incorrect password, aborting");
    }
}

pub fn create_key(salt: String, password: String) -> Result<Key, &'static str> {
    if let Some(salt) = Salt::from_slice(salt.as_bytes()) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_key_creation() {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();

        let key_ok = create_key(salt, password);

        assert!(key_ok.is_ok());
    }

    #[test]
    fn test_create_upstream_url() {
        let req = TestRequest::default()
            .uri("https://proxy.com/bucket/file.zip?p1=ok1&p2=ok2")
            .param("name", "bucket/file.zip") // hack to force parsing
            .to_http_request();

        assert_eq!(
            default_config("https://upstream.com").create_upstream_url(&req),
            "https://upstream.com/bucket/file.zip?p1=ok1&p2=ok2"
        );

        assert_eq!(
            default_config("https://upstream.com/").create_upstream_url(&req),
            "https://upstream.com/bucket/file.zip?p1=ok1&p2=ok2"
        );

        assert_eq!(
            default_config("https://upstream.com/sub_folder/").create_upstream_url(&req),
            "https://upstream.com/sub_folder/bucket/file.zip?p1=ok1&p2=ok2"
        );

        let req = TestRequest::default()
            .uri("https://proxy.com/bucket/file.zip")
            .param("name", "bucket/file.zip") // hack to force parsing
            .to_http_request();

        assert_eq!(
            default_config("https://upstream.com").create_upstream_url(&req),
            "https://upstream.com/bucket/file.zip"
        );
    }

    fn default_config(upstream_base_url: &str) -> Config {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();

        Config {
            key: create_key(salt, password).unwrap(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            upstream_base_url: Some(upstream_base_url.to_string()),
            noop: false,
            input_file: None,
            output_file: None,
            address: "127.0.0.1:1234".to_socket_addrs().unwrap().next(),
            max_connections: 1,
        }
    }
}
