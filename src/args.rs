use serde::Deserialize;

pub const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>]
  ds_proxy decrypt <input-file> <output-file> [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>]
  ds_proxy proxy [--address=<address>] [--password-file=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--upstream-url=<upstream-url>] [--noop]
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
    pub flag_noop: bool,
    pub arg_output_file: Option<String>,
    pub flag_password_file: Option<String>,
    pub flag_salt: Option<String>,
    pub flag_upstream_url: Option<String>,
    pub cmd_encrypt: bool,
    pub cmd_decrypt: bool,
    pub cmd_proxy: bool,
}
