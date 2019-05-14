extern crate encrypt;

use docopt::Docopt;
use serde::Deserialize;
use encrypt::config::Config;

const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_fs_cipher encrypt <input-file> <output-file> [--noop=<arg>]
  ds_fs_cipher decrypt <input-file> <output-file> [--noop=<arg>]
  ds_fs_cipher (-h | --help)
  ds_fs_cipher --version

Options:
  -h --help     Show this screen.
  --version     Show version.
  --noop=<arg>  If true, will not do any encryption or decryption [default: false]
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_input_file: Option<String>,
    arg_output_file: Option<String>,
    cmd_encrypt: bool,
    cmd_decrypt: bool,
    flag_noop: String,
}

impl Args {
    fn update_config(&self, config: &mut Config) {
      config.noop = self.flag_noop == "true";
    }
}

fn main() {
    let mut config = Config::new();
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    args.update_config(&mut config);

    println!("{:#?}", config);
    
    if args.cmd_encrypt {
        encrypt::file::encrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap(), &config);
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(args.arg_input_file.unwrap(), args.arg_output_file.unwrap(), &config);
    }
}
