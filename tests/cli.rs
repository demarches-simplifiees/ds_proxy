use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use encrypt::header::{PREFIX, PREFIX_SIZE};
use std::path::Path;
use std::process::{Child, Command, Output};

const PASSWORD: &'static str = "plop";
const SALT: &'static str = "12345678901234567890123456789012";
const HASH_FILE_ARG: &'static str = "--hash-file=tests/fixtures/password.hash";
const CHUNK_SIZE: &'static str = "512"; //force multiple pass

#[test]
fn encrypt_and_decrypt() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "plop";
    let salt = "12345678901234567890123456789012";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";
    let chunk_size = "512"; //force multiple pass

    let original = "tests/fixtures/computer.svg";
    let encrypted = temp.child("computer.svg.enc");
    let decrypted = temp.child("computer.dec.svg");

    let encrypted_path = encrypted.path();
    let decrypted_path = decrypted.path();

    let mut encrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    encrypt_cmd
        .arg("encrypt")
        .arg(original)
        .arg(encrypted_path)
        .arg(hash_file_arg)
        .env("DS_PASSWORD", password)
        .env("DS_SALT", salt)
        .env("DS_CHUNK_SIZE", chunk_size);

    encrypt_cmd.assert().success();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(encrypted_path)
        .arg(decrypted_path)
        .arg(hash_file_arg)
        .env("DS_PASSWORD", password)
        .env("DS_SALT", salt)
        .env("DS_CHUNK_SIZE", chunk_size);

    decrypt_cmd.assert().success();

    let original_bytes = std::fs::read(original).unwrap();
    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();

    temp.close().unwrap();

    assert_eq!(original_bytes, decrypted_bytes);
}

#[test]
fn decrypting_a_plaintext_file_yields_the_original_file() {
    let temp = assert_fs::TempDir::new().unwrap();

    let original = "tests/fixtures/computer.svg";
    let encrypted = "tests/fixtures/computer.svg.enc";
    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(encrypted)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT);

    decrypt_cmd.assert().success();

    let original_bytes = std::fs::read(original).unwrap();
    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();

    temp.close().unwrap();

    assert_eq!(original_bytes, decrypted_bytes);
}

#[test]
fn the_app_crashes_on_a_missing_password() {
    let temp = assert_fs::TempDir::new().unwrap();

    let salt = "12345678901234567890123456789012";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";

    let encrypted = "tests/fixtures/computer.svg.enc";
    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(encrypted)
        .arg(decrypted_path)
        .arg(hash_file_arg)
        .env("DS_SALT", salt);

    decrypt_cmd.assert().failure();
}

#[test]
fn the_app_crashes_on_a_missing_hash() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "plop";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";

    let encrypted = "tests/fixtures/computer.svg.enc";
    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(encrypted)
        .arg(decrypted_path)
        .arg(hash_file_arg)
        .env("DS_PASSWORD", password);

    decrypt_cmd.assert().failure();
}

#[test]
fn the_app_crashes_with_an_invalid_password() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "this is not the expected password";

    let encrypted = "tests/fixtures/computer.svg.enc";
    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(encrypted)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", password)
        .env("DS_SALT", SALT);

    decrypt_cmd.assert().failure();
}

use std::{thread, time};

#[test]
fn end_to_end_upload_and_download_node() {
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

    let mut proxy_server = launch_proxy(3000);
    let mut node_server = launch_node();

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
        .kill()
        .expect("killing the proxy server should succeed !");
    node_server
        .kill()
        .expect("killing node's upload server should succeed !");
    temp.close().unwrap();
}

fn launch_proxy(upstream_port: i32) -> Child {
    Command::cargo_bin("ds_proxy")
        .unwrap()
        .arg("proxy")
        .arg("--address=localhost:4444")
        .arg(format!("--upstream-url=http://localhost:{}", upstream_port))
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE)
        .spawn()
        .expect("failed to execute ds_proxy")
}

fn launch_node() -> Child {
    Command::new("node")
        .arg("tests/fixtures/server-static/server.js")
        .spawn()
        .expect("failed to execute node")
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
