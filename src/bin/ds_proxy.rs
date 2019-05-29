extern crate encrypt;
extern crate sodiumoxide;
extern crate log;
extern crate env_logger;

use docopt::Docopt;
use encrypt::config::Config;
use serde::Deserialize;
use sodiumoxide::crypto::pwhash::argon2i13::{pwhash_verify, HashedPassword};
use log::info;

const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file> <password>
  ds_proxy decrypt <input-file> <output-file> <password>
  ds_proxy proxy <listen-adress> <listen-port> <password> [--noop]
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help             Show this screen.
  --version             Show version.
";

#[derive(Debug, Deserialize, Clone)]
struct Args {
    arg_input_file: Option<String>,
    arg_output_file: Option<String>,
    arg_listen_adress: Option<String>,
    arg_password: Option<String>,
    arg_listen_port: Option<u16>,
    cmd_encrypt: bool,
    cmd_decrypt: bool,
    cmd_proxy: bool,
    flag_noop: bool,
}

fn create_config(args: &Args) -> Config {
    Config{
        password: Some(args.arg_password.clone().unwrap()),
        noop: args.flag_noop,
        ..Config::new_from_env()
    }
}

fn main() {
    env_logger::init();

    let args : Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let config: Config = create_config(&args);

    if args.cmd_proxy {
        if args.flag_noop {
            info!("proxy in dry mode")
        }

        let serialized_hash = std::fs::read("hash.key")
            .expect("Unable to read hash file");
        let hash = HashedPassword::from_slice(&serialized_hash[..]);
        if pwhash_verify(&hash.unwrap(), config.password.clone().unwrap().trim_end().as_bytes()) {
            println!("This password matches the saved hash, starting the proxy");

            let listen_adress = args.clone().arg_listen_adress.unwrap();
            let listen_port = args.arg_listen_port.unwrap();
            let _ = encrypt::proxy::main(&listen_adress, listen_port, config);
        } else {
            println!("Incorrect password, aborting")
        }
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(
            args.clone().arg_input_file.unwrap(),
            args.clone().arg_output_file.unwrap(),
            &config,
        );
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(
            args.clone().arg_input_file.unwrap(),
            args.clone().arg_output_file.unwrap(),
            &config,
        );
    }
}

