extern crate encrypt;
extern crate sodiumoxide;

use docopt::Docopt;
use encrypt::config::Config;
use serde::Deserialize;
use sodiumoxide::crypto::pwhash::argon2i13::pwhash;
use sodiumoxide::crypto::pwhash::argon2i13::pwhash_verify;
use sodiumoxide::crypto::pwhash::argon2i13::HashedPassword;
use sodiumoxide::crypto::pwhash::argon2i13::MEMLIMIT_INTERACTIVE;
use sodiumoxide::crypto::pwhash::argon2i13::OPSLIMIT_INTERACTIVE;
use std::io;

const USAGE: &str = "
DS encryption proxy.

Usage:
  ds_proxy encrypt <input-file> <output-file>
  ds_proxy decrypt <input-file> <output-file>
  ds_proxy proxy <listen-adress> <listen-port> [<noop>]
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
    arg_listen_port: Option<u16>,
    cmd_encrypt: bool,
    cmd_decrypt: bool,
    cmd_proxy: bool,
    flag_noop: Option<String>,
}

fn main() {
    let config: Config = Config::new_from_env();
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_proxy {
        let mut password = String::new();
        println!("What password do you want to use?");
        io::stdin()
            .read_line(&mut password)
            .expect("unable to read the password");
        let serialized_hash = std::fs::read("hash.key").expect("Unable to read hash file");
        let hash = HashedPassword::from_slice(&serialized_hash[..]);
        println!(
            "This password matches the saved hash {}",
            pwhash_verify(&hash.unwrap(), password.trim_end().as_bytes())
        );

        let listen_adress = &args.arg_listen_adress.unwrap();
        let listen_port = args.arg_listen_port.unwrap();
        let upstream_base_url = "https://storage.gra5.cloud.ovh.net".to_string();
        encrypt::proxy::main(listen_adress, listen_port, upstream_base_url, config).unwrap();
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(
            args.arg_input_file.unwrap(),
            args.arg_output_file.unwrap(),
            config,
        );
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(
            args.arg_input_file.unwrap(),
            args.arg_output_file.unwrap(),
            config,
        );
    }
}

