//! Windows refuses an ordinary file in place of the link socket.
use super::*;

#[test]
fn a_windows_link_path_must_be_a_socket_in_a_private_folder() {
    let folder = private_folder();
    let file = folder.path().join("link.sock");
    std::fs::write(&file, b"not a socket").unwrap();
    assert!(matches!(
        HubClient::over_link(&file, TIMEOUT),
        Err(HubError::LinkSocket(LinkSocketError::Exposed))
    ));
}
