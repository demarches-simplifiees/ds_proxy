use super::{args, keyring::Keyring};
use actix_web::HttpRequest;

use sodiumoxide::crypto::pwhash::argon2i13::{pwhash_verify, HashedPassword};

use std::env;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use url::Url;

// match nginx default (proxy_buffer_size in ngx_stream_proxy_module)
pub const DEFAULT_CHUNK_SIZE: usize = 16 * 1024;
pub const DEFAULT_LOCAL_ENCRYPTION_DIRECTORY: &str = "ds_proxy/local_encryption/";

pub enum Config {
    Decrypt(DecryptConfig),
    Encrypt(EncryptConfig),
    Http(HttpConfig),
}

#[derive(Debug, Clone)]
pub struct DecryptConfig {
    pub keyring: Keyring,
    pub input_file: String,
    pub output_file: String,
}

#[derive(Debug, Clone)]
pub struct EncryptConfig {
    pub keyring: Keyring,
    pub chunk_size: usize,
    pub input_file: String,
    pub output_file: String,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    pub upstream_base_url: String,
    pub noop: bool,
    pub keyring: Keyring,
    pub chunk_size: usize,
    pub max_connections: usize,
    pub address: SocketAddr,
    pub local_encryption_directory: PathBuf,
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

        let keyring = Keyring::load(salt, password).unwrap();

        if args.cmd_encrypt {
            Config::Encrypt(EncryptConfig {
                keyring,
                chunk_size,
                input_file: args.arg_input_file.clone().unwrap(),
                output_file: args.arg_output_file.clone().unwrap(),
            })
        } else if args.cmd_decrypt {
            Config::Decrypt(DecryptConfig {
                keyring,
                input_file: args.arg_input_file.clone().unwrap(),
                output_file: args.arg_output_file.clone().unwrap(),
            })
        } else {
            let local_encryption_directory = match &args.flag_local_encryption_directory {
                Some(directory) => PathBuf::from(directory),
                None => match env::var("DS_LOCAL_ENCRYPTION_DIRECTORY") {
                    Ok(directory) => PathBuf::from(directory),
                    _ => {
                        let mut path_buf = PathBuf::new();
                        path_buf.push(env::temp_dir());
                        path_buf.push(DEFAULT_LOCAL_ENCRYPTION_DIRECTORY);
                        path_buf
                    }
                },
            };

            std::fs::create_dir_all(local_encryption_directory.clone()).unwrap_or_else(|why| {
                panic!(
                    "Cannot create tmp directory {:?}: {}",
                    local_encryption_directory, why
                )
            });

            let upstream_base_url = match &args.flag_upstream_url {
                Some(upstream_url) => Some(upstream_url.to_string()),
                None => Some(env::var("DS_UPSTREAM_URL").expect(
                    "Missing upstream_url, use DS_UPSTREAM_URL env or --upstream-url cli argument",
                )),
            }
            .unwrap();

            let address = match &args.flag_address {
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
            .unwrap();

            Config::Http(HttpConfig {
                keyring,
                chunk_size,
                upstream_base_url,
                noop: args.flag_noop,
                address,
                local_encryption_directory,
                max_connections: args.flag_max_connections.unwrap_or(25_000),
            })
        }
    }
}

impl HttpConfig {
    pub fn create_upstream_url(&self, req: &HttpRequest) -> String {
        let base = Url::parse(self.upstream_base_url.as_ref()).unwrap();
        let mut url = base.join(&req.match_info()["name"]).unwrap();

        if !req.query_string().is_empty() {
            url.set_query(Some(req.query_string()));
        }

        url.to_string()
    }

    pub fn local_encryption_path_for(&self, req: &HttpRequest) -> PathBuf {
        let name = req.match_info().get("name").unwrap();
        let mut filepath = self.local_encryption_directory.clone();
        filepath.push(name);
        filepath
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_key_creation() {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();

        let keyring = Keyring::load(salt, password);

        assert!(keyring.is_ok());
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

    fn default_config(upstream_base_url: &str) -> HttpConfig {
        let password = "Correct Horse Battery Staple".to_string();
        let salt = "abcdefghabcdefghabcdefghabcdefgh".to_string();

        HttpConfig {
            keyring: Keyring::load(salt, password).unwrap(),
            chunk_size: DEFAULT_CHUNK_SIZE,
            upstream_base_url: upstream_base_url.to_string(),
            noop: false,
            address: "127.0.0.1:1234".to_socket_addrs().unwrap().next().unwrap(),
            max_connections: 1,
            local_encryption_directory: PathBuf::from(DEFAULT_LOCAL_ENCRYPTION_DIRECTORY),
        }
    }
}
