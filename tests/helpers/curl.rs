use std::process::{Command, Output};
use std::{thread, time};

pub fn curl_get_status(url: &str) -> String {
    let stdout = Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .arg("-o")
        .arg("/dev/null")
        .arg("-s")
        .arg("-w")
        .arg("%{http_code}")
        .output()
        .expect("failed to perform download")
        .stdout;

    std::str::from_utf8(&stdout).unwrap().to_string()
}

pub fn curl_put(file_path: &str, url: &str) -> Output {
    let cmd = Command::new("curl")
        .arg("-XPUT")
        .arg(url)
        .arg("--data-binary")
        .arg(format!("@{}", file_path))
        .output()
        .expect("failed to perform upload");

    // add sleep to let node write the file on the disk
    thread::sleep(time::Duration::from_millis(100));

    cmd
}

pub fn curl_get_content_length_header(url: &str) -> usize {
    let response = curl_get_headers(url);

    response
        .split("\r\n")
        .find(|x| x.starts_with("content-length"))
        .unwrap()
        .replace("content-length: ", "")
        .parse::<usize>()
        .ok()
        .unwrap()
}

pub fn curl_get_headers(url: &str) -> std::string::String {
    let response = Command::new("curl")
        .arg("-I")
        .arg("-XGET")
        .arg(url)
        .output()
        .expect("failed to perform download")
        .stdout;

    String::from_utf8_lossy(&response).to_string()
}

pub fn curl_get(url: &str) -> Output {
    Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .output()
        .expect("failed to perform download")
}

pub fn curl_range_get(url: &str, range_start: usize, range_end: usize) -> Output {
    let range_arg = format!("Range: bytes={}-{}", range_start, range_end);

    Command::new("curl")
        .arg("-XGET")
        .arg(url)
        .arg("-H")
        .arg(range_arg)
        .arg("-vv")
        .output()
        .expect("failed to perform download")
}

pub fn curl_socket_get(url: &str) -> Output {
    Command::new("curl")
        .arg("-XGET")
        .arg("--unix-socket")
        .arg("/tmp/actix-uds.socket")
        .arg(url)
        .output()
        .expect("failed to perform download")
}
