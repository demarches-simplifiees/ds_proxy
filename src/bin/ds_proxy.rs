extern crate encrypt;

use docopt::Docopt;
use serde::Deserialize;

const USAGE: &'static str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file>
  ds_proxy decrypt <input-file> <output-file>
  ds_proxy proxy <listen-adress> <listen-port>
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help     Show this screen.
  --version     Show version.
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
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());


    if args.cmd_proxy {
        encrypt::proxy::main(&args.arg_listen_adress.unwrap(), args.arg_listen_port.unwrap()).unwrap();
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap());
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap());
    }
}
