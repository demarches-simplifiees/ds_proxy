mod helpers;
pub use helpers::*;

use actix_web::web::Bytes;
use std::path::Path;

#[test]
#[serial(servers)]
fn local_encryption() {
    let uploaded_path = Path::new("/tmp/ds_proxy/local_encryption/archive.zip");

    ensure_is_absent(uploaded_path.to_str().unwrap());

    let _proxy_node_and_redis = ProxyAndNode::start();

    curl_put(
        COMPUTER_SVG_PATH,
        "localhost:4444/local/encrypt/archive.zip",
    );

    assert!(uploaded_path.exists());

    let curl_download = curl_get("localhost:4444/local/encrypt/archive.zip");

    assert!(!uploaded_path.exists());

    let buf = decrypt_bytes(Bytes::from(curl_download.stdout));

    assert_eq!(buf, COMPUTER_SVG_BYTES);
}

#[test]
#[serial(servers)]
fn fetch_missing_file() {
    let _proxy_node_and_redis = ProxyAndNode::start();
    assert_eq!(
        curl_get_status("localhost:4444/local/fetch/missing_file"),
        "404"
    );
}
