extern crate encrypt;
extern crate sodiumoxide;
#[macro_use]
extern crate lazy_static;
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
  ds_proxy encrypt <input-file> <output-file>
  ds_proxy decrypt <input-file> <output-file>
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

fn create_config() -> Config {
    Config{
        password: Some(ARGS.arg_password.clone().unwrap()),
        noop: ARGS.flag_noop,
        ..Config::new_from_env()
    }
}

lazy_static! {
    static ref CONFIG: Config = create_config();
    static ref ARGS: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
}

fn main() {
    env_logger::init();

    if ARGS.cmd_proxy {
        if ARGS.flag_noop {
            info!("proxy in dry mode")
        }

        let serialized_hash = std::fs::read("hash.key")
            .expect("Unable to read hash file");
        let hash = HashedPassword::from_slice(&serialized_hash[..]);
        if pwhash_verify(&hash.unwrap(), CONFIG.password.clone().unwrap().trim_end().as_bytes()) {
            println!("This password matches the saved hash, starting the proxy");

            let listen_adress = ARGS.clone().arg_listen_adress.unwrap();
            let listen_port = ARGS.arg_listen_port.unwrap();
            let _ = encrypt::proxy::main(&listen_adress, listen_port, &CONFIG);
        } else {
            println!("Incorrect password, aborting")
        }
    } else if ARGS.cmd_encrypt {
        encrypt::file::encrypt(
            ARGS.clone().arg_input_file.unwrap(),
            ARGS.clone().arg_output_file.unwrap(),
            &CONFIG,
        );
    } else if ARGS.cmd_decrypt {
        encrypt::file::decrypt(
            ARGS.clone().arg_input_file.unwrap(),
            ARGS.clone().arg_output_file.unwrap(),
            &CONFIG,
        );
    }
}

