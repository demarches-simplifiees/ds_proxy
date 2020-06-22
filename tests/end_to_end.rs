use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use encrypt::header::{PREFIX, PREFIX_SIZE};
use std::path::Path;
use std::process::{Child, Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{thread, time};
use uuid::Uuid;

const PASSWORD: &'static str = "plop";
const SALT: &'static str = "12345678901234567890123456789012";
const HASH_FILE_ARG: &'static str = "--hash-file=tests/fixtures/password.hash";
const CHUNK_SIZE: &'static str = "512"; //force multiple pass

#[test]
fn end_to_end_upload_and_download() {
    /*
    This test:
     - spawns a node server that stores uploaded files in tests/fixtures/server-static/uploads/
     - spawns a ds proxy that uses the node proxy as a storage backend
     - uploads a file using curl via the DS proxy
     - checks that said file is encrypted
     - decrypt the uploaded file by the decrypted command and check the result
     - downloads the uploaded file via the proxy, and checks that its content matches the initial content
    */
    let original_path = "tests/fixtures/computer.svg";
    let original_bytes = std::fs::read(original_path).unwrap();
    let uploaded_path = "tests/fixtures/server-static/uploads/victory";

    let temp = assert_fs::TempDir::new().unwrap();
    let decrypted_file = temp.child("computer.dec.svg");
    let decrypted_path = decrypted_file.path();

    if Path::new(uploaded_path).exists() {
        std::fs::remove_file(uploaded_path)
            .expect(&format!("Unable to remove {} !", uploaded_path.to_owned()));
    }

    let mut proxy_server = launch_proxy(PrintServerLogs::No);
    let mut node_server = launch_node(PrintServerLogs::No);
    thread::sleep(time::Duration::from_millis(1000));

    let curl_upload = curl_put(original_path, "localhost:4444/victory");
    if !curl_upload.status.success() {
        panic!("unable to upload file !");
    }

    let uploaded_bytes = std::fs::read(uploaded_path).expect("uploaded should exist !");
    assert_eq!(&uploaded_bytes[0..PREFIX_SIZE], PREFIX);

    decrypt(uploaded_path, decrypted_path);
    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();
    assert_eq!(original_bytes, decrypted_bytes);

    let curl_download = curl_get("localhost:4444/victory");
    assert_eq!(curl_download.stdout, original_bytes);

    let curl_socket_download = curl_socket_get("localhost:4444/victory");
    assert_eq!(curl_socket_download.stdout, original_bytes);

    let curl_chunked_download = curl_get("localhost:4444/chunked/victory");
    assert_eq!(curl_chunked_download.stdout, original_bytes);

    proxy_server
        .child
        .kill()
        .expect("killing the proxy server should succeed !");
    node_server
        .child
        .kill()
        .expect("killing node's upload server should succeed !");
    temp.close().unwrap();
}

#[test]
fn concurent_uploads() {
    /*
    This test:
     - spawns a node server that stores uploaded files in tests/fixtures/server-static/uploads/
     - spawns a ds proxy that uses the node proxy as a storage backend
     - attempts to store many files concurently files, while the server has a high latency

    For large number of threads, you may need to increase the open files limit before this works.
    For instance:
        ulimit -n 2048
    */

    const THREADS_COUNT: u16 = 20;
    const DELAY_BETWEEN_TREADS: Duration = Duration::from_millis(10);
    const SERVER_LATENCY: Duration = Duration::from_millis(100);

    let mut proxy_server = launch_proxy(PrintServerLogs::No);
    let mut node_server = launch_node_with_latency(Some(SERVER_LATENCY), PrintServerLogs::No);
    thread::sleep(Duration::from_secs(1));

    // Spawn threads (with a slight delay between each)
    let mut child_threads = vec![];
    let counter = Arc::new(Mutex::new(0));

    for _ in 0..THREADS_COUNT {
        thread::sleep(DELAY_BETWEEN_TREADS);

        let counter = Arc::clone(&counter);

        let name = format!("thread {}", child_threads.len());
        let child_thread = thread::Builder::new()
            .name(name)
            .spawn(move || {
                {
                    let mut threads_count = counter.lock().unwrap();
                    *threads_count += 1;
                    println!("Number of threads: {}", threads_count);
                }

                let original_path = "tests/fixtures/computer.svg";
                let original_bytes = std::fs::read(original_path).unwrap();

                let stored_filename = Uuid::new_v4();
                let stored_file_url = format!("localhost:4444/{}", stored_filename);
                let uploaded_path =
                    format!("tests/fixtures/server-static/uploads/{}", stored_filename);

                let temp = assert_fs::TempDir::new().unwrap();
                let decrypted_file = temp.child("computer.dec.svg");
                let decrypted_path = decrypted_file.path();

                let curl_upload = curl_put(original_path, &stored_file_url);
                if !curl_upload.status.success() {
                    panic!("unable to upload file!");
                }

                let curl_upload = curl_put(original_path, &stored_file_url);
                if !curl_upload.status.success() {
                    panic!("unable to upload file!");
                }

                let uploaded_bytes =
                    std::fs::read(&uploaded_path).expect("uploaded should exist !");
                assert!(uploaded_bytes.len() > 0);
                assert_eq!(&uploaded_bytes[0..PREFIX_SIZE], PREFIX);

                decrypt(&uploaded_path, decrypted_path);
                let decrypted_bytes = std::fs::read(decrypted_path).unwrap();
                assert_eq!(original_bytes.len(), decrypted_bytes.len());
                assert_eq!(original_bytes, decrypted_bytes);

                let curl_download = curl_get(&stored_file_url);
                assert_eq!(curl_download.stdout.len(), original_bytes.len());
                assert_eq!(curl_download.stdout, original_bytes);

                let curl_socket_download = curl_socket_get(&stored_file_url);
                assert_eq!(curl_socket_download.stdout.len(), original_bytes.len());
                assert_eq!(curl_socket_download.stdout, original_bytes);

                let curl_chunked_download =
                    curl_get(&format!("localhost:4444/chunked/{}", stored_filename));
                assert_eq!(curl_chunked_download.stdout.len(), original_bytes.len());
                assert_eq!(curl_chunked_download.stdout, original_bytes);

                // Cleanup
                temp.close().unwrap();
                std::fs::remove_file(&uploaded_path)
                    .expect(&format!("Unable to remove uploaded file{}!", uploaded_path));

                {
                    let mut threads_count = counter.lock().unwrap();
                    *threads_count -= 1;
                    println!("Number of threads: {}", threads_count);
                }
            })
            .unwrap();
        child_threads.push(child_thread);
    }

    // Wait for all threads to have successfully finished
    // (or panic if a child thread panicked.)
    for child_thread in child_threads {
        child_thread.join().expect("A child thread panicked");
    }

    proxy_server
        .child
        .kill()
        .expect("killing the proxy server should succeed !");
    node_server
        .child
        .kill()
        .expect("killing node's upload server should succeed !");
}

//
// Test helpers
//

// Ensure a child process is killed when it goes out of scope.
// This avoids leaving running processes around when a test fails.
struct ChildGuard {
    child: Child,
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
enum PrintServerLogs {
    Yes,
    No,
}

fn launch_proxy(log: PrintServerLogs) -> ChildGuard {
    let mut command = Command::cargo_bin("ds_proxy").unwrap();
    command
        .arg("proxy")
        .arg("--address=localhost:4444")
        .arg("--upstream-url=http://localhost:3000")
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE);

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

fn launch_node(log: PrintServerLogs) -> ChildGuard {
    launch_node_with_latency(None, log)
}

fn launch_node_with_latency(latency: Option<Duration>, log: PrintServerLogs) -> ChildGuard {
    let mut command = Command::new("node");
    command.arg("tests/fixtures/server-static/server.js");

    match log {
        PrintServerLogs::Yes => {
            command.env("DEBUG", "express:*");
        }
        PrintServerLogs::No => (),
    }

    match latency {
        Some(l) => {
            command.arg(format!("--latency={}", l.as_millis()));
        }
        None => (),
    }

    let child = command.spawn().expect("failed to execute node");
    ChildGuard {
        child,
        description: "node",
    }
}

fn curl_put(file_path: &str, url: &str) -> Output {
    Command::new("curl")
        .arg("-XPUT")
        .arg(url)
        .arg("--data-binary")
        .arg(format!("@{}", file_path))
        .output()
        .expect("failed to perform upload")
}

fn curl_get(url: &str) -> Output {
    Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .output()
        .expect("failed to perform download")
}

fn curl_socket_get(url: &str) -> Output {
    Command::new("curl")
        .arg("-XGET")
        .arg("--unix-socket")
        .arg("/tmp/actix-uds.socket")
        .arg(url)
        .output()
        .expect("failed to perform download")
}

fn decrypt(encrypted_path: &str, decrypted_path: &std::path::Path) -> assert_cmd::assert::Assert {
    Command::cargo_bin("ds_proxy")
        .unwrap()
        .arg("decrypt")
        .arg(encrypted_path)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE)
        .assert()
        .success()
}
