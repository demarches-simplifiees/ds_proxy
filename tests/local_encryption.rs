mod helpers;
pub use helpers::*;

use actix_web::web::Bytes;

#[test]
#[serial(servers)]
fn local_encryption() {
    let original_path = "tests/fixtures/computer.svg";
    let original_bytes = std::fs::read(original_path).unwrap();

    let _proxy_and_node = ProxyAndNode::start();

    curl_put(original_path, "localhost:4444/local/encrypt/archive.zip");

    let curl_download = curl_get("localhost:4444/local/fetch/archive.zip");

    let buf = decrypt_bytes(Bytes::from(curl_download.stdout));

    assert_eq!(buf, original_bytes);
}

#[test]
#[serial(servers)]
fn fetch_missing_file() {
    let _proxy_and_node = ProxyAndNode::start();
    assert_eq!(
        curl_get_status("localhost:4444/local/fetch/missing_file"),
        "404"
    );
}
