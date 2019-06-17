extern crate encrypt;
extern crate sodiumoxide;
extern crate log;
extern crate env_logger;

use docopt::Docopt;
use encrypt::config::Config;
use log::info;
use encrypt::args::{Args, USAGE};

fn main() {
    env_logger::init();
    sodiumoxide::init().unwrap();

    let args : Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let config: Config = Config::create_config(&args);

    if args.cmd_proxy {
        if args.flag_noop {
            info!("proxy in dry mode")
        }

        let listen_adress = args.clone().arg_listen_adress.unwrap();
        let listen_port = args.arg_listen_port.unwrap();
        let _ = encrypt::proxy::main(&listen_adress, listen_port, config);
    } else if args.cmd_encrypt {
        encrypt::file::encrypt(config);
    } else if args.cmd_decrypt {
        encrypt::file::decrypt(config);
    }
}

