use assert_fs::prelude::*;
use ds_proxy::crypto::header::*;
use serial_test::serial;
use std::path::Path;

mod helpers;
pub use helpers::*;

#[test]
#[serial(servers)]
fn upload_and_download() {
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
            .unwrap_or_else(|_| panic!("Unable to remove {} !", uploaded_path.to_owned()));
    }

    let _proxy_and_node = ProxyAndNode::start();

    curl_put(original_path, "localhost:4444/upstream/victory");

    let uploaded_bytes = std::fs::read(uploaded_path).expect("uploaded should exist !");
    assert_eq!(&uploaded_bytes[0..PREFIX_SIZE], PREFIX);

    decrypt(uploaded_path, decrypted_path);
    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();
    assert_eq!(original_bytes, decrypted_bytes);

    let curl_download = curl_get("localhost:4444/upstream/victory");
    assert_eq!(curl_download.stdout, original_bytes);

    let curl_range_download = curl_range_get("localhost:4444/upstream/victory", 0, 10);
    assert_eq!(curl_range_download.stdout, &original_bytes[0..11]);

    let curl_socket_download = curl_socket_get("localhost:4444/upstream/victory");
    assert_eq!(curl_socket_download.stdout, original_bytes);

    let curl_chunked_download = curl_get("localhost:4444/upstream/chunked/victory");
    assert_eq!(curl_chunked_download.stdout, original_bytes);

    temp.close().unwrap();
}
