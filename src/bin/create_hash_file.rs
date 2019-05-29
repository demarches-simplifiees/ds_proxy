extern crate encrypt;
extern crate sodiumoxide;

use docopt::Docopt;
use serde::Deserialize;
use sodiumoxide::crypto::pwhash::argon2i13::{pwhash, MEMLIMIT_INTERACTIVE, OPSLIMIT_INTERACTIVE};
use std::io;

const USAGE: &str = "
Generates a hash file for use with the encryption proxy.

Usage:
  create_hash_file <output-file>
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_output_file: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    let mut password = String::new();
    let filename: String = args.arg_output_file.unwrap();
    let path = std::path::Path::new(&filename);

    if !path.exists() {
        println!("What password do you want to use?");
        io::stdin()
            .read_line(&mut password)
            .expect("unable to read the password");

        let hashed_password = pwhash(
            password.trim_end().as_bytes(),
            OPSLIMIT_INTERACTIVE,
            MEMLIMIT_INTERACTIVE,
        );

        let original_hashed_bytes = hashed_password.as_ref().unwrap();
        std::fs::write(filename, original_hashed_bytes).expect("Unable to write hash file");
        println!("Hash correctly saved");
    } else {
        eprintln!("{} already exists, please provide another file", filename);
    }
}
