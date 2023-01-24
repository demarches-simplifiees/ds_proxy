use std::convert::TryInto;

use assert_fs::prelude::*;
use ds_proxy::crypto::header;
use serial_test::serial;

mod helpers;
pub use helpers::*;

#[test]
#[serial(servers)]
fn multiple_keys() {
    /* this tests encrypt 3 files with 3 differents keys
     * ensure the right key_id is written in the file
     * and then download and decrypt the differnts files
     */

    let temp = assert_fs::TempDir::new().unwrap();

    let ids_keys: Vec<(u64, [u8; 32])> = (0..3).map(|id| (id, random_key())).collect();

    for (id, _) in &ids_keys {
        println!("trying with id: {}", id);
        let keyring_file = temp.child(format!("keyring_{}", id));
        let keyring_path = keyring_file.path().to_str().unwrap();

        let id_usize: usize = *id as usize;

        let keys = ids_keys
            .iter()
            .take(id_usize + 1)
            .map(|(_, key)| key.clone())
            .collect();

        make_keyring(keyring_path, keys);

        let upload_url = format!("localhost:4444/upstream/victory_{}", id);
        let upload_path = format!("tests/fixtures/server-static/uploads/victory_{}", id);

        ensure_is_absent(&upload_path);

        let _proxy_and_node = ProxyAndNode::start_with_keyring_path(keyring_path);
        curl_put(COMPUTER_SVG_PATH, &upload_url);

        assert_eq!(&key_id(&upload_path), id);
    }

    let final_keyring_file = temp.child("final_keyring");
    let final_keyring_path = final_keyring_file.path().to_str().unwrap();
    let final_keys = ids_keys.iter().map(|(_, key)| key.clone()).collect();
    make_keyring(final_keyring_path, final_keys);

    let _proxy_and_node = ProxyAndNode::start_with_keyring_path(final_keyring_path);

    for i in 0..3 {
        let download_url = format!("localhost:4444/upstream/victory_{}", i);
        let curl_download = curl_get(&download_url);
        assert_eq!(curl_download.stdout, COMPUTER_SVG_BYTES);
    }

    temp.close().unwrap();
}

fn make_keyring(keyring_path: &str, keys: Vec<[u8; 32]>) {
    ds_proxy::keyring_utils::encrypt_and_save_keyring(
        keys,
        keyring_path,
        PASSWORD.to_string(),
        SALT.to_string(),
    )
}

fn key_id(uploaded_path: &str) -> u64 {
    let uploaded_bytes = std::fs::read(uploaded_path).expect("uploaded should exist !");

    u64::from_le_bytes(
        uploaded_bytes[header::HEADER_SIZE..header::HEADER_V2_SIZE]
            .try_into()
            .unwrap(),
    )
}

fn random_key() -> [u8; 32] {
    sodiumoxide::randombytes::randombytes(32)
        .try_into()
        .unwrap()
}
