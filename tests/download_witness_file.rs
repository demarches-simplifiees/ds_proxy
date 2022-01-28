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
    let original_path = "tests/fixtures/computer.svg";
    let original_bytes = std::fs::read(original_path).unwrap();

    let encrypted_path = "tests/fixtures/computer.svg.enc";
    let uploaded_path = "tests/fixtures/server-static/uploads/computer.svg.enc";

    std::fs::copy(encrypted_path, uploaded_path).expect("copy failed");

    let _proxy_and_node = ProxyAndNode::start();

    let curl_download = curl_get("localhost:4444/upstream/computer.svg.enc");
    if !curl_download.status.success() {
        panic!("unable to download file !");
    }

    assert_eq!(curl_download.stdout, original_bytes);

    let content_length =
        curl_get_content_length_header("http://localhost:4444/upstream/computer.svg.enc");

    let metadata = std::fs::metadata(original_path).unwrap();
    assert_eq!(metadata.len(), content_length as u64);

    let headers = curl_get_headers("http://localhost:4444/upstream/computer.svg.enc");
    let transfert_encoding = headers
        .split("\r\n")
        .find(|x| x.starts_with("transfer-encoding"));

    assert_eq!(None, transfert_encoding);
}
