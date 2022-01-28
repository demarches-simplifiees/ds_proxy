use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use std::process::Command;

mod helpers;
pub use helpers::*;

const PASSWORD: &str = "plop";
const SALT: &str = "12345678901234567890123456789012";
const HASH_FILE_ARG: &str = "--hash-file=tests/fixtures/password.hash";

#[test]
fn encrypt_and_decrypt() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "plop";
    let salt = "12345678901234567890123456789012";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";
    let chunk_size = "512"; //force multiple pass

    let encrypted = temp.child("computer.svg.enc");
    let decrypted = temp.child("computer.dec.svg");

    let encrypted_path = encrypted.path();
    let decrypted_path = decrypted.path();

    let mut encrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    encrypt_cmd
        .arg("encrypt")
        .arg(COMPUTER_SVG_PATH)
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

    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();

    temp.close().unwrap();

    assert_eq!(COMPUTER_SVG_BYTES, decrypted_bytes);
}

#[test]
fn decrypt_witness_file() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "plop";
    let salt = "12345678901234567890123456789012";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(hash_file_arg)
        .env("DS_PASSWORD", password)
        .env("DS_SALT", salt);

    decrypt_cmd.assert().success();

    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();

    temp.close().unwrap();

    assert_eq!(decrypted_bytes, COMPUTER_SVG_BYTES);
}

#[test]
fn decrypting_a_plaintext_file_yields_the_original_file() {
    let temp = assert_fs::TempDir::new().unwrap();

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT);

    decrypt_cmd.assert().success();

    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();

    temp.close().unwrap();

    assert_eq!(decrypted_bytes, COMPUTER_SVG_BYTES);
}

#[test]
fn the_app_crashes_on_a_missing_password() {
    let temp = assert_fs::TempDir::new().unwrap();

    let salt = "12345678901234567890123456789012";
    let hash_file_arg = "--hash-file=tests/fixtures/password.hash";

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
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

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(hash_file_arg)
        .env("DS_PASSWORD", password);

    decrypt_cmd.assert().failure();
}

#[test]
fn the_app_crashes_with_an_invalid_password() {
    let temp = assert_fs::TempDir::new().unwrap();

    let password = "this is not the expected password";

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", password)
        .env("DS_SALT", SALT);

    decrypt_cmd.assert().failure();
}
