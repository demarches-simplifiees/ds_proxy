extern crate encrypt;

use docopt::Docopt;
use serde::Deserialize;
use encrypt::proxy::Config;

const USAGE: &'static str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> [--noop=<arg>]
  ds_proxy decrypt <input-file> <output-file> [--noop=<arg>]
  ds_proxy proxy <listen-adress> <listen-port> [--noop=<arg>]
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help     Show this screen.
  --version     Show version.
  --noop=<arg>  If true, will not do any encryption or decryption [default: false]
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_input_file: Option<String>,
    arg_output_file: Option<String>,
    arg_listen_adress: Option<String>,
    arg_listen_port: Option<u16>,
    cmd_encrypt: bool,
    cmd_decrypt: bool,
    cmd_proxy: bool,
    flag_noop: String,
}

impl Args {
    fn noop(&self) -> bool {
        self.flag_noop == "true"
    }

    fn update_config(&self, config: &mut Config) {
      config.listen_adress = self.arg_listen_adress.clone();
      config.listen_port = self.arg_listen_port;
      config.noop = self.noop();
    }
}

fn main() {
    let mut config = Config::new();
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    args.update_config(&mut config);

    if args.cmd_proxy {
        let listen_adress = &args.arg_listen_adress.unwrap();
        let listen_port = args.arg_listen_port.unwrap();
        let upstream_base_url = "https://storage.gra5.cloud.ovh.net".to_string();
        let _ = encrypt::proxy::main(listen_adress, listen_port, upstream_base_url, config.noop);
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap(), config.noop);
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap(), config.noop);
    }
}
