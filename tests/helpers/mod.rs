use assert_cmd::prelude::*;
use std::process::{Child, Command};
use std::time::Duration;

mod curl;
pub use curl::*;

const PASSWORD: &str = "plop";
const SALT: &str = "12345678901234567890123456789012";
const HASH_FILE_ARG: &str = "--hash-file=tests/fixtures/password.hash";
pub const CHUNK_SIZE: usize = 512;

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


pub fn launch_node(log: PrintServerLogs) -> ChildGuard {
    launch_node_with_latency(None, log)
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

#[allow(dead_code)]
pub enum PrintServerLogs {
    Yes,
    No,
}


pub fn decrypt(encrypted_path: &str, decrypted_path: &std::path::Path) -> assert_cmd::assert::Assert {
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
