use serial_test::serial;
use std::path::Path;
use std::{thread, time};

mod helpers;
pub use helpers::*;

#[test]
#[serial(servers)]
fn ping() {
    /*
    This test:
     - spawns a ds proxy
     - curl /ping and expect to fetch a 200
     - add a maintenance file
     - curl /ping and expect to fetch a 404 which should trigger a maintenance mode
       on a upper stream proxy
    */
    let mut proxy_server = launch_proxy(PrintServerLogs::No);
    thread::sleep(time::Duration::from_millis(2000));

    let maintenance_file_path = "maintenance";

    if Path::new(maintenance_file_path).exists() {
        std::fs::remove_file(maintenance_file_path)
            .unwrap_or_else(|_| panic!("Unable to remove {} !", maintenance_file_path.to_owned()));
    }

    assert_eq!(curl_get_status("localhost:4444/ping"), "200");

    std::fs::File::create(maintenance_file_path)
        .unwrap_or_else(|_| panic!("Unable to create {} !", maintenance_file_path.to_owned()));

    assert_eq!(curl_get_status("localhost:4444/ping"), "404");

    std::fs::remove_file(maintenance_file_path)
        .unwrap_or_else(|_| panic!("Unable to remove {} !", maintenance_file_path.to_owned()));

    proxy_server
        .child
        .kill()
        .expect("killing the proxy server should succeed !");
}
