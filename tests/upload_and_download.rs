use assert_fs::prelude::*;
use ds_proxy::crypto::header::*;
use serial_test::serial;

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
    let uploaded_path = "tests/fixtures/server-static/uploads/jail/cell/victory";

    let temp = assert_fs::TempDir::new().unwrap();
    let decrypted_file = temp.child("computer.dec.svg");
    let decrypted_path = decrypted_file.path();

    ensure_is_absent(uploaded_path);

    let _proxy_and_node = ProxyAndNode::start();

    curl_put(COMPUTER_SVG_PATH, "localhost:4444/upstream/victory");

    let uploaded_bytes = std::fs::read(uploaded_path).expect("uploaded should exist !");
    assert_eq!(&uploaded_bytes[0..PREFIX_SIZE], PREFIX);

    decrypt(uploaded_path, decrypted_path);
    let decrypted_bytes = std::fs::read(decrypted_path).unwrap();
    assert_eq!(decrypted_bytes, COMPUTER_SVG_BYTES);

    let curl_download = curl_get("localhost:4444/upstream/victory");
    assert_eq!(curl_download.stdout, COMPUTER_SVG_BYTES);

    let curl_range_download = curl_range_get("localhost:4444/upstream/victory", 0, 10);
    assert_eq!(curl_range_download.stdout, &COMPUTER_SVG_BYTES[0..11]);

    let curl_socket_download = curl_socket_get("localhost:4444/upstream/victory");
    assert_eq!(curl_socket_download.stdout, COMPUTER_SVG_BYTES);

    let curl_chunked_download = curl_get("localhost:4444/upstream/victory?chunked=true");
    assert_eq!(curl_chunked_download.stdout, COMPUTER_SVG_BYTES);

    temp.close().unwrap();
}
