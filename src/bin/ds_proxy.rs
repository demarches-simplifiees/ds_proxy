extern crate encrypt;
extern crate sodiumoxide;
extern crate log;
extern crate env_logger;

use docopt::Docopt;
use encrypt::config::Config;
use sodiumoxide::crypto::pwhash::argon2i13::{pwhash_verify, HashedPassword};
use log::{info, error};
use encrypt::args::{Args, USAGE};

fn main() {
    env_logger::init();
    sodiumoxide::init().unwrap();

    let args : Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let config: Config = Config::create_config(&args);

    match std::fs::read("hash.key") {
        Err(_) => {
            error!("hash.key not found");
            std::process::exit(1);
        },
        Ok(file) => {
            let hash = HashedPassword::from_slice(&file[..]);

            if !pwhash_verify(&hash.unwrap(), config.password.clone().trim_end().as_bytes()) {
                error!("Incorrect password, aborting");
                std::process::exit(1);
            }

        }
    }

    if args.cmd_proxy {
        if args.flag_noop {
            info!("proxy in dry mode")
        }

        let listen_adress = args.clone().arg_listen_adress.unwrap();
        let listen_port = args.arg_listen_port.unwrap();
        let _ = encrypt::proxy::main(&listen_adress, listen_port, config);
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

