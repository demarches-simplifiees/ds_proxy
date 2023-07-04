use serial_test::serial;

mod helpers;
pub use helpers::*;

#[test]
#[serial(servers)]
fn download_witness_file() {
    /*
    This test:
     - spawns a node server that stores uploaded files in tests/fixtures/server-static/uploads/
     - spawns a ds proxy that uses the node proxy as a storage backend
     - copy a witness file in the right directory to be downloaded
     - downloads the uploaded file via the proxy, and checks that its content matches the initial content
    */
    let uploaded_path = "tests/fixtures/server-static/uploads/jail/cell/computer.svg.enc";

    std::fs::copy(ENCRYPTED_COMPUTER_SVG_PATH, uploaded_path).expect("copy failed");

    let _proxy_and_node = ProxyAndNode::start();

    let curl_download = curl_get("localhost:4444/upstream/computer.svg.enc");

    assert_eq!(curl_download.stdout, COMPUTER_SVG_BYTES);

    let content_length =
        curl_get_content_length_header("http://localhost:4444/upstream/computer.svg.enc");

    let metadata = std::fs::metadata(COMPUTER_SVG_PATH).unwrap();
    assert_eq!(metadata.len(), content_length as u64);

    let headers = curl_get_headers("http://localhost:4444/upstream/computer.svg.enc");
    let transfert_encoding = headers
        .split("\r\n")
        .find(|x| x.starts_with("transfer-encoding"));

    assert_eq!(None, transfert_encoding);
}
