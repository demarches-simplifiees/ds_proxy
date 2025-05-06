use serde::Deserialize;

pub const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--keyring-file=<keyring-file>]
  ds_proxy decrypt <input-file> <output-file> [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--keyring-file=<keyring-file>]
  ds_proxy proxy [--address=<address>] [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--upstream-url=<upstream-url>] [--local-encryption-directory=<local-encryption-directory>] [--write-once] [--keyring-file=<keyring-file>] [--aws-access-key=<aws-access-key>] [--aws-secret-key=<aws-secret-key>] [--aws-region=<aws-region>] [--backend-connection-timeout=<backend-connection-timeout>] [--redis-url=<redis-url>] [--redis-timeout-wait=<redis-timeout-wait>] [--redis-timeout-create=<redis-timeout-create>] [--redis-timeout-recycle=<redis-timeout-recycle>] [--redis-pool-max-size=<redis-pool-max-size>]
  ds_proxy add-key [--password-file=<password-file>] [--salt=<salt>] [--keyring-file=<keyring-file>]
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
    pub arg_input_file: Option<String>,
    pub flag_keyring_file: Option<String>,
    pub arg_output_file: Option<String>,
    pub flag_password_file: Option<String>,
    pub flag_salt: Option<String>,
    pub flag_upstream_url: Option<String>,
    pub flag_local_encryption_directory: Option<String>,
    pub flag_aws_access_key: Option<String>,
    pub flag_aws_secret_key: Option<String>,
    pub flag_aws_region: Option<String>,
    pub flag_backend_connection_timeout: Option<u64>,
    pub cmd_encrypt: bool,
    pub cmd_decrypt: bool,
    pub cmd_proxy: bool,
    pub cmd_add_key: bool,
    pub flag_redis_url: Option<String>,
    pub flag_write_once: bool,
    pub flag_redis_timeout_wait: Option<u64>,
    pub flag_redis_timeout_create: Option<u64>,
    pub flag_redis_timeout_recycle: Option<u64>,
    pub flag_redis_pool_max_size: Option<usize>,
}
