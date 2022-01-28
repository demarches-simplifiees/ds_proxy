use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs::read;
use std::process::Command;

mod helpers;
pub use helpers::*;

#[test]
fn encrypt_and_decrypt() {
    let temp = TempDir::new().unwrap();

    let encrypted = temp.child("computer.svg.enc");
    let decrypted = temp.child("computer.dec.svg");

    let encrypted_path = encrypted.path();
    let decrypted_path = decrypted.path();

    let mut encrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    encrypt_cmd
        .arg("encrypt")
        .arg(COMPUTER_SVG_PATH)
        .arg(encrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE.to_string())
        .assert()
        .success();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(encrypted_path)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .env("DS_CHUNK_SIZE", CHUNK_SIZE.to_string())
        .assert()
        .success();

    let decrypted_bytes = read(decrypted_path).unwrap();

    assert_eq!(COMPUTER_SVG_BYTES, decrypted_bytes);
}

#[test]
fn decrypt_witness_file() {
    let temp = TempDir::new().unwrap();

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("decrypt")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD)
        .env("DS_SALT", SALT)
        .assert()
        .success();

    let decrypted_bytes = read(decrypted_path).unwrap();

    assert_eq!(decrypted_bytes, COMPUTER_SVG_BYTES);
}

#[test]
fn the_app_crashes_on_a_missing_password() {
    let temp = TempDir::new().unwrap();

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_SALT", SALT);

    decrypt_cmd.assert().failure();
}

#[test]
fn the_app_crashes_on_a_missing_hash() {
    let temp = TempDir::new().unwrap();

    let decrypted = temp.child("computer.dec.svg");
    let decrypted_path = decrypted.path();

    let mut decrypt_cmd = Command::cargo_bin("ds_proxy").unwrap();
    decrypt_cmd
        .arg("proxy")
        .arg(ENCRYPTED_COMPUTER_SVG_PATH)
        .arg(decrypted_path)
        .arg(HASH_FILE_ARG)
        .env("DS_PASSWORD", PASSWORD);

    decrypt_cmd.assert().failure();
}

#[test]
fn the_app_crashes_with_an_invalid_password() {
    let temp = TempDir::new().unwrap();

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
