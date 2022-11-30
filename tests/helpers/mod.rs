pub use serial_test::serial;

use actix_web::web::{BufMut, Bytes, BytesMut};
use actix_web::Error;
use assert_cmd::prelude::*;
use ds_proxy::config::create_keys;
use futures::executor::block_on_stream;
use std::path::Path;
use std::process::{Child, Command};
use std::time::Duration;
use std::{thread, time};

use ds_proxy::crypto::*;

mod curl;
pub use curl::*;

pub const PASSWORD: &str = "plop";
pub const SALT: &str = "12345678901234567890123456789012";
pub const HASH_FILE_ARG: &str = "--hash-file=tests/fixtures/password.hash";
pub const CHUNK_SIZE: usize = 512;

pub const COMPUTER_SVG_PATH: &str = "tests/fixtures/computer.svg";
pub static COMPUTER_SVG_BYTES: Bytes =
    Bytes::from_static(include_bytes!("../fixtures/computer.svg"));

pub const ENCRYPTED_COMPUTER_SVG_PATH: &str = "tests/fixtures/computer.svg.enc";
pub static ENCRYPTED_COMPUTER_SVG_BYTES: Bytes =
    Bytes::from_static(include_bytes!("../fixtures/computer.svg.enc"));

#[allow(dead_code)]
pub struct ProxyAndNode {
    proxy: ChildGuard,
    node: ChildGuard,
}

impl ProxyAndNode {
    pub fn start() -> ProxyAndNode {
        ProxyAndNode::start_with_options(None, PrintServerLogs::No)
    }

    pub fn start_with_options(latency: Option<Duration>, log: PrintServerLogs) -> ProxyAndNode {
        let proxy = launch_proxy(log);
        let node = launch_node_with_latency(latency, log);
        thread::sleep(time::Duration::from_secs(4));
        ProxyAndNode { proxy, node }
    }
}

pub fn launch_proxy(log: PrintServerLogs) -> ChildGuard {
    let mut command = Command::cargo_bin("ds_proxy").unwrap();
    command
        .arg("proxy")
        .arg("--address=localhost:4444")
        .arg("--upstream-url=http://localhost:3333")
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE.to_string());

    match log {
        PrintServerLogs::Yes => {
            command.env("RUST_LOG", "trace");
        }
        PrintServerLogs::No => (),
    }

    let child = command.spawn().expect("failed to execute ds_proxy");
    ChildGuard {
        child,
        description: "ds_proxy",
    }
}

pub fn launch_node_with_latency(latency: Option<Duration>, log: PrintServerLogs) -> ChildGuard {
    let mut command = Command::new("node");
    command.arg("tests/fixtures/server-static/server.js");

    match log {
        PrintServerLogs::Yes => {
            command.env("DEBUG", "express:*");
        }
        PrintServerLogs::No => (),
    }

    if let Some(l) = latency {
        command.arg(format!("--latency={}", l.as_millis()));
    }

    let child = command.spawn().expect("failed to execute node");
    ChildGuard {
        child,
        description: "node",
    }
}

pub struct ChildGuard {
    pub child: Child,
    description: &'static str,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        match self.child.kill() {
            Err(e) => println!(
                "ChildGuard: could not kill out-of-scope '{}' process: {}",
                self.description, e
            ),
            Ok(_) => println!(
                "ChildGuard: successfully killed out-of-scope '{}' process",
                self.description
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PrintServerLogs {
    Yes,
    No,
}

pub fn decrypt(
    encrypted_path: &str,
    decrypted_path: &std::path::Path,
) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ds_proxy")
        .unwrap()
        .arg("decrypt")
        .arg(encrypted_path)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE.to_string())
        .assert()
        .success()
}

pub fn decrypt_bytes(input: Bytes) -> BytesMut {
    let source: Result<Bytes, Error> = Ok(input);
    let source_stream = futures::stream::once(Box::pin(async { source }));
    let keyring = create_keys(SALT.to_string(), PASSWORD.to_string()).unwrap();
    let decoder = Decoder::new(keyring, Box::new(source_stream));

    block_on_stream(decoder)
        .map(|r| r.unwrap())
        .fold(BytesMut::with_capacity(64), |mut acc, x| {
            acc.put(x);
            acc
        })
}

pub fn ensure_is_absent(file_path: &str) {
    if Path::new(file_path).exists() {
        std::fs::remove_file(file_path)
            .unwrap_or_else(|_| panic!("Unable to remove {} !", file_path));
    }
}
