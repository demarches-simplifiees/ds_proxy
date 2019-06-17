use serde::Deserialize;

pub const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> [--password=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>]
  ds_proxy decrypt <input-file> <output-file> [--password=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>]
  ds_proxy proxy <listen-adress> <listen-port> [--password=<password-file>] [--salt=<salt>] [--chunk-size=<chunk-size>] [--upstream-url=<upstream-url>] [--noop]
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help             Show this screen.
  --version             Show version.
";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Args {
    pub arg_input_file: Option<String>,
    pub arg_output_file: Option<String>,
    pub arg_listen_adress: Option<String>,
    pub arg_password_file: Option<String>,
    pub arg_salt: Option<String>,
    pub arg_chunk_size: Option<usize>,
    pub arg_upstream_url: Option<String>,
    pub arg_listen_port: Option<u16>,
    pub cmd_encrypt: bool,
    pub cmd_decrypt: bool,
    pub cmd_proxy: bool,
    pub flag_noop: bool,
}
