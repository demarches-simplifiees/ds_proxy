use std::os::unix::fs::PermissionsExt;
use std::{fs, thread, time};

mod helpers;
pub use helpers::*;

#[test]
#[serial(servers)]
fn socket_is_created_with_0660_permissions() {
    /*
    This test:
     - spawns a ds proxy listening on a unix socket
     - checks that the socket file is created with 0660 permissions
       (read/write for the owner and the group, nothing for others)
    */
    let _proxy_server = launch_proxy(PrintServerLogs::No, None, None, false);
    thread::sleep(time::Duration::from_secs(2));

    let metadata = fs::metadata(UNIX_SOCKET_PATH)
        .unwrap_or_else(|_| panic!("the unix socket {} should exist", UNIX_SOCKET_PATH));
    let mode = metadata.permissions().mode() & 0o777;

    assert_eq!(mode, 0o660, "expected socket mode 0660, got {:o}", mode);
}
