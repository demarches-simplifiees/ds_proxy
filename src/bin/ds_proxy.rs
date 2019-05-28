extern crate encrypt;
extern crate sodiumoxide;
#[macro_use]
extern crate lazy_static;

use docopt::Docopt;
use encrypt::config::Config;
use serde::Deserialize;
use sodiumoxide::crypto::pwhash::argon2i13::{pwhash_verify, HashedPassword};
use std::io;
use std::sync::Mutex;

const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file>
  ds_proxy decrypt <input-file> <output-file>
  ds_proxy proxy <listen-adress> <listen-port> <password> [<noop>]
  ds_proxy (-h | --help)
  ds_proxy --version

Options:
  -h --help             Show this screen.
  --version             Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_input_file: Option<String>,
    arg_output_file: Option<String>,
    arg_listen_adress: Option<String>,
    arg_password: Option<String>,
    arg_listen_port: Option<u16>,
    cmd_encrypt: bool,
    cmd_decrypt: bool,
    cmd_proxy: bool,
    flag_noop: Option<String>,
}

lazy_static! {
    static ref CONFIG: Config = Config::new_from_env();
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_proxy {
        // CONFIG.password = args.arg_password.clone();
        /*let mut password = String::new();
        println!("What password do you want to use?");
        io::stdin()
            .read_line(&mut password)
            .expect("unable to read the password");*/
        let serialized_hash = std::fs::read("hash.key").expect("Unable to read hash file");
        let hash = HashedPassword::from_slice(&serialized_hash[..]);
        if pwhash_verify(&hash.unwrap(), CONFIG.password.clone().unwrap().trim_end().as_bytes()) {
            println!("This password matches the saved hash, starting the proxy");

            let listen_adress = &args.arg_listen_adress.unwrap();
            let listen_port = args.arg_listen_port.unwrap();
            let _ = encrypt::proxy::main(listen_adress, listen_port, &CONFIG);
        } else {
            println!("Incorrect password, aborting")
        }
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(
            args.arg_input_file.unwrap(),
            args.arg_output_file.unwrap(),
            &CONFIG,
        );
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(
            args.arg_input_file.unwrap(),
            args.arg_output_file.unwrap(),
            &CONFIG,
        );
    }
}

