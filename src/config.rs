use super::{args, keyring::Keyring, keyring_utils::load_keyring};
use actix_web::HttpRequest;

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
    AddKeyConfig(AddKeyConfig),
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
    pub upstream_base_url: Url,
    pub keyring: Keyring,
    pub chunk_size: usize,
    pub address: SocketAddr,
    pub local_encryption_directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AddKeyConfig {
    pub password: String,
    pub salt: String,
    pub keyring_file: String,
}

impl Config {
    pub fn create_config(args: &args::Args) -> Config {
        let password = match &args.flag_password_file {
            Some(password_file) => read_file_content(password_file),
            None => env::var("DS_PASSWORD")
                .expect("Missing password, use DS_PASSWORD env or --password-file cli argument"),
        };

        let salt = match &args.flag_salt {
            Some(salt) => salt.to_string(),
            None => {
                env::var("DS_SALT").expect("Missing salt, use DS_SALT env or --salt cli argument")
            }
        };

        let keyring_file: String = match &args.flag_keyring_file {
            Some(keyring_file) => keyring_file.to_string(),
            None => env::var("DS_KEYRING")
                .expect("Missing keyring, use DS_KEYRING env or --keyring-file cli argument"),
        };

        if args.cmd_add_key {
            return Config::AddKeyConfig(AddKeyConfig {
                password,
                salt,
                keyring_file,
            });
        }

        let chunk_size = match &args.flag_chunk_size {
            Some(chunk_size) => *chunk_size,
            None => match env::var("DS_CHUNK_SIZE") {
                Ok(chunk_str) => chunk_str.parse::<usize>().unwrap_or(DEFAULT_CHUNK_SIZE),
                _ => DEFAULT_CHUNK_SIZE,
            },
        };

        let keyring = load_keyring(&keyring_file, password, salt);

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

            let raw_upstream_base_url = match &args.flag_upstream_url {
                Some(upstream_url) => Some(upstream_url.to_string()),
                None => Some(env::var("DS_UPSTREAM_URL").expect(
                    "Missing upstream_url, use DS_UPSTREAM_URL env or --upstream-url cli argument",
                )),
            }
            .unwrap();

            let upstream_base_url = normalize_and_parse_upstream_url(raw_upstream_base_url);

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
                address,
                local_encryption_directory,
            })
        }
    }
}

// ensure upstream_url ends with a "/ to avoid
// upstream url: "https://upstream/dir"
// request: "https://proxy/file"
// "https://upstream/dir".join('file') => https://upstream/file
// instead ".../upstream/dir/".join('file') => https://upstream/dir/file
fn normalize_and_parse_upstream_url(mut url: String) -> Url {
    if !url.ends_with('/') {
        url.push('/');
    }
    Url::parse(&url).unwrap()
}

impl HttpConfig {
    pub fn create_upstream_url(&self, req: &HttpRequest) -> Option<String> {
        // Warning: join process '../'
        // "https://a.com/jail/".join('../escape') => "https://a.com/escape"
        let mut url = self
            .upstream_base_url
            .join(&req.match_info()["name"])
            .unwrap();

        if self.is_traversal_attack(&url) {
            return None;
        }

        if !req.query_string().is_empty() {
            url.set_query(Some(req.query_string()));
        }

        Some(url.to_string())
    }

    pub fn local_encryption_path_for(&self, req: &HttpRequest) -> PathBuf {
        let name = req.match_info().get("name").unwrap();
        let mut filepath = self.local_encryption_directory.clone();
        filepath.push(name);
        filepath
    }

    fn is_traversal_attack(&self, url: &Url) -> bool {
        // https://upstream.com => [Some("")]
        // https://upstream.com/jail/cell/ => [Some("jail"), Some("cell"), Some("")]
        let mut base_segments: Vec<&str> =
            self.upstream_base_url.path_segments().unwrap().collect();

        // remove the last segment corresponding to "/"
        base_segments.pop();

        let mut url_segments = url.path_segments().unwrap();

        // ensure that all segment of the upstream_base_url
        // are present in the final url
        let safe = base_segments.iter().all(|base_segment| {
            let url_segment = url_segments.next().unwrap();
            base_segment == &url_segment
        });

        !safe
    }
}

fn read_file_content(path_string: &str) -> String {
    match std::fs::read(path_string) {
        Err(why) => panic!("couldn't open {}: {}", path_string, why),
        Ok(file) => String::from_utf8(file).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_normalize_and_parse_upstream_url() {
        assert_eq!(
            normalize_and_parse_upstream_url("https://upstream.com/dir".to_string()),
            Url::parse("https://upstream.com/dir/").unwrap()
        );
    }

    #[test]
    fn test_create_upstream_url() {
        let base = "https://upstream.com/";
        let jailed_base = "https://upstream.com/jail/cell/";

        let config = default_config(base);
        let jailed_config = default_config(jailed_base);

        let file = TestRequest::default()
            .uri("https://proxy.com/file")
            .param("name", "file") // hack to force parsing
            .to_http_request();

        assert_eq!(
            config.create_upstream_url(&file),
            Some("https://upstream.com/file".to_string())
        );

        assert_eq!(
            jailed_config.create_upstream_url(&file),
            Some("https://upstream.com/jail/cell/file".to_string())
        );

        let sub_dir_file = TestRequest::default()
            .uri("https://proxy.com/sub/dir/file")
            .param("name", "sub/dir/file") // hack to force parsing
            .to_http_request();

        assert_eq!(
            config.create_upstream_url(&sub_dir_file),
            Some("https://upstream.com/sub/dir/file".to_string())
        );

        assert_eq!(
            jailed_config.create_upstream_url(&sub_dir_file),
            Some("https://upstream.com/jail/cell/sub/dir/file".to_string())
        );

        let path_traversal_file = TestRequest::default()
            .uri("https://proxy.com/../escape")
            .param("name", "../escape") // hack to force parsing
            .to_http_request();

        assert_eq!(
            config.create_upstream_url(&path_traversal_file),
            Some("https://upstream.com/escape".to_string())
        );

        assert_eq!(
            jailed_config.create_upstream_url(&path_traversal_file),
            None
        );

        let file_with_query_string = TestRequest::default()
            .uri("https://proxy.com/bucket/file.zip?p1=ok1&p2=ok2")
            .param("name", "bucket/file.zip") // hack to force parsing
            .to_http_request();

        assert_eq!(
            config.create_upstream_url(&file_with_query_string),
            Some("https://upstream.com/bucket/file.zip?p1=ok1&p2=ok2".to_string())
        );
    }

    fn default_config(upstream_base_url: &str) -> HttpConfig {
        let keyring = Keyring::new(HashMap::new());

        HttpConfig {
            keyring,
            chunk_size: DEFAULT_CHUNK_SIZE,
            upstream_base_url: normalize_and_parse_upstream_url(upstream_base_url.to_string()),
            address: "127.0.0.1:1234".to_socket_addrs().unwrap().next().unwrap(),
            local_encryption_directory: PathBuf::from(DEFAULT_LOCAL_ENCRYPTION_DIRECTORY),
        }
    }
}
