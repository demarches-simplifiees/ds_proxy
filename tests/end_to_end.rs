use actix_web::client::Client;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use ds_proxy::header::HEADER_SIZE;
use ds_proxy::header::{PREFIX, PREFIX_SIZE};
use serial_test::serial;
use sodiumoxide::crypto::secretstream::xchacha20poly1305::{ABYTES, HEADERBYTES};
use std::path::Path;
use std::process::{Child, Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{thread, time};
use uuid::Uuid;

const PASSWORD: &'static str = "plop";
const SALT: &'static str = "12345678901234567890123456789012";
const HASH_FILE_ARG: &'static str = "--hash-file=tests/fixtures/password.hash";
const CHUNK_SIZE: usize = 512;

#[test]
#[serial(servers)]
fn ping() {
    /*
    This test:
     - spawns a ds proxy
     - curl /ping and expect to fetch a 200
     - add a maintenance file
     - curl /ping and expect to fetch a 404 which should trigger a maintenance mode
       on a upper stream proxy
    */
    let mut proxy_server = launch_proxy(PrintServerLogs::No);
    thread::sleep(time::Duration::from_millis(2000));

    let maintenance_file_path = "maintenance";

    if Path::new(maintenance_file_path).exists() {
        std::fs::remove_file(maintenance_file_path).expect(&format!(
            "Unable to remove {} !",
            maintenance_file_path.to_owned()
        ));
    }

    assert_eq!(curl_get_status("localhost:4444/ping"), "200");

    std::fs::File::create(maintenance_file_path).expect(&format!(
        "Unable to create {} !",
        maintenance_file_path.to_owned()
    ));

    assert_eq!(curl_get_status("localhost:4444/ping"), "404");

    std::fs::remove_file(maintenance_file_path).expect(&format!(
        "Unable to remove {} !",
        maintenance_file_path.to_owned()
    ));

    proxy_server
        .child
        .kill()
        .expect("killing the proxy server should succeed !");
}

#[actix_rt::test]
#[serial(servers)]
async fn test_content_length_and_transfert_encoding() {
    let _proxy_server = launch_proxy(PrintServerLogs::No);
    let _node_server = launch_node(PrintServerLogs::No);
    thread::sleep(time::Duration::from_millis(4000));

    let tmp_dir = assert_fs::TempDir::new().unwrap();

    // multiple of chunk size
    let nb_chunk = 2;
    let original_length = nb_chunk * CHUNK_SIZE;
    let content = vec![0; original_length];

    let expected_encrypted_length = HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + CHUNK_SIZE);

    let (uploaded_length, downloaded_length) =
        uploaded_and_downloaded_content_length(&content).await;

    assert_eq!(expected_encrypted_length, uploaded_length);
    assert_eq!(original_length, downloaded_length);

    // not a multiple of chunk size
    let nb_chunk = 2;
    let original_length = nb_chunk * CHUNK_SIZE + 1;
    let content = vec![0; original_length];

    let expected_encrypted_length =
        HEADER_SIZE + HEADERBYTES + nb_chunk * (ABYTES + CHUNK_SIZE) + ABYTES + 1;

    let (uploaded_length, downloaded_length) =
        uploaded_and_downloaded_content_length(&content).await;

    assert_eq!(expected_encrypted_length, uploaded_length);
    assert_eq!(original_length, downloaded_length);

    tmp_dir.close().unwrap();
}

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
struct TestHeaders {
    #[serde(rename = "content-length")]
    content_length: String,
}

async fn uploaded_and_downloaded_content_length(content: &[u8]) -> (usize, usize) {
    let client = Client::new();

    client
        .put("http://localhost:4444/file")
        .send_body(actix_web::dev::Body::from_slice(content))
        .await
        .unwrap();

    let mut response = client
        .get("http://localhost:3333/last_put_headers")
        .send()
        .await
        .unwrap();

    let deserialized: TestHeaders =
        serde_json::from_slice(&response.body().await.unwrap()).unwrap();

    let response = client
        .get("http://localhost:4444/file")
        .send()
        .await
        .unwrap();

    let downloaded_length = response
        .headers()
        .get(actix_web::http::header::CONTENT_LENGTH)
        .and_then(|l| l.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();

    (
        deserialized.content_length.parse::<usize>().unwrap(),
        downloaded_length,
    )
}

#[actix_rt::test]
#[serial(servers)]
async fn download_witness_file() {
    /*
    This test:
     - spawns a node server that stores uploaded files in tests/fixtures/server-static/uploads/
     - spawns a ds proxy that uses the node proxy as a storage backend
     - copy a witness file in the right directory to be downloaded
     - downloads the uploaded file via the proxy, and checks that its content matches the initial content
    */
    let original_path = "tests/fixtures/computer.svg";
    let original_bytes = std::fs::read(original_path).unwrap();

    let encrypted_path = "tests/fixtures/computer.svg.enc";
    let uploaded_path = "tests/fixtures/server-static/uploads/computer.svg.enc";

    std::fs::copy(encrypted_path, uploaded_path).expect("copy failed");

    let mut proxy_server = launch_proxy(PrintServerLogs::No);
    let mut node_server = launch_node(PrintServerLogs::No);
    thread::sleep(time::Duration::from_millis(4000));

    let curl_download = curl_get("localhost:4444/computer.svg.enc");
    if !curl_download.status.success() {
        panic!("unable to download file !");
    }

    assert_eq!(curl_download.stdout, original_bytes);

    use actix_web::client::Client;
    let client = Client::new();

    let response = client
        .get("http://localhost:4444/computer.svg.enc")
        .send()
        .await
        .unwrap();

    let content_length = response
        .headers()
        .get(actix_web::http::header::CONTENT_LENGTH)
        .and_then(|l| l.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap();

    let metadata = std::fs::metadata(original_path).unwrap();
    assert_eq!(metadata.len(), content_length);

    let transfert_encoding = response
        .headers()
        .get(actix_web::http::header::TRANSFER_ENCODING);
    assert_eq!(None, transfert_encoding);

    proxy_server
        .child
        .kill()
        .expect("killing the proxy server should succeed !");
    node_server
        .child
        .kill()
        .expect("killing node's upload server should succeed !");
}

#[test]
#[serial(servers)]
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
    thread::sleep(time::Duration::from_millis(4000));

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

    let curl_range_download = curl_range_get("localhost:4444/victory", 0, 10);
    assert_eq!(curl_range_download.stdout, &original_bytes[0..11]);

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
#[serial(servers)]
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
    thread::sleep(Duration::from_secs(4));

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
    let cmd = Command::new("curl")
        .arg("-XPUT")
        .arg(url)
        .arg("--data-binary")
        .arg(format!("@{}", file_path))
        .output()
        .expect("failed to perform upload");

    // add sleep to let node write the file on the disk
    thread::sleep(time::Duration::from_millis(100));

    cmd
}

fn curl_get(url: &str) -> Output {
    Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .output()
        .expect("failed to perform download")
}

fn curl_range_get(url: &str, range_start: usize, range_end: usize) -> Output {
    let range_arg = format!("Range: bytes={}-{}", range_start, range_end);

    Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .arg("-H")
        .arg(range_arg)
        .arg("-vv")
        .output()
        .expect("failed to perform download")
}

fn curl_get_status(url: &str) -> String {
    let stdout = Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .arg("-o")
        .arg("/dev/null")
        .arg("-s")
        .arg("-w")
        .arg("%{http_code}")
        .output()
        .expect("failed to perform download")
        .stdout;

    std::str::from_utf8(&stdout).unwrap().to_string()
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
        .env("DS_CHUNK_SIZE", CHUNK_SIZE.to_string())
        .assert()
        .success()
}
