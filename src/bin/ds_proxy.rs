extern crate ds_proxy;
extern crate env_logger;
extern crate log;
extern crate sodiumoxide;

use docopt::Docopt;
use ds_proxy::args::{Args, USAGE};
use ds_proxy::config::Config;
use log::info;
use std::env;

fn main() {
    env_logger::init();

    if let Ok(url) = env::var("DS_PROXY_SENTRY_URL") {
        info!("Sentry will be notified on {}", url);
        let _guard = sentry::init(url);
    }

    sodiumoxide::init().unwrap();

    let docopt: Docopt = Docopt::new(USAGE)
        .unwrap_or_else(|e| e.exit())
        .version(Some(env!("GIT_HASH").to_string()));

    let args: Args = docopt.deserialize().unwrap_or_else(|e| e.exit());

    let config: Config = Config::create_config(&args);

    if args.cmd_proxy {
        if args.flag_noop {
            info!("proxy in dry mode")
        }

        ds_proxy::http::main(config).unwrap();
    } else if args.cmd_encrypt {
        ds_proxy::file::encrypt(config);
    } else if args.cmd_decrypt {
        ds_proxy::file::decrypt(config);
    }
}
