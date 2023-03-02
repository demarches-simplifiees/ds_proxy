use serde::Deserialize;

pub const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> [--password-file=<password-file>] [--hash-file=<hash-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--keyring-file=<keyring-file>]
  ds_proxy decrypt <input-file> <output-file> [--password-file=<password-file>] [--hash-file=<hash-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--keyring-file=<keyring-file>]
  ds_proxy proxy [--address=<address>] [--password-file=<password-file>] [--hash-file=<hash-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--upstream-url=<upstream-url>] [--max-connections=<max-connections>] [--local-encryption-directory=<local-encryption-directory>] [--noop] [--keyring-file=<keyring-file>]
  ds_proxy bootstrap-keyring [--password-file=<password-file>] [--hash-file=<hash-file>] [--salt=<salt>]
  ds_proxy add-key [--password-file=<password-file>] [--hash-file=<hash-file>] [--salt=<salt>] [--keyring-file=<keyring-file>]
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help             Show this screen.
  --version             Show version.
";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Args {
    pub flag_address: Option<String>,
    pub flag_chunk_size: Option<usize>,
    pub flag_hash_file: Option<String>,
    pub arg_input_file: Option<String>,
    pub flag_keyring_file: Option<String>,
    pub flag_noop: bool,
    pub arg_output_file: Option<String>,
    pub flag_password_file: Option<String>,
    pub flag_salt: Option<String>,
    pub flag_upstream_url: Option<String>,
    pub flag_max_connections: Option<usize>,
    pub flag_local_encryption_directory: Option<String>,
    pub cmd_encrypt: bool,
    pub cmd_decrypt: bool,
    pub cmd_proxy: bool,
    pub cmd_bootstrap_keyring: bool,
    pub cmd_add_key: bool,
}
