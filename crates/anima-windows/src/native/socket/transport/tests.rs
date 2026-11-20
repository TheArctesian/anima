use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn private_socket_exchanges_bytes_and_cancelled_accept_can_be_reused() {
    let root = tempfile::tempdir().unwrap();
    crate::files::protect(root.path()).unwrap();
    let path = root.path().join("local.sock");
    let listener = Listener::bind(&path).unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
    let exchange = async {
        let (client, server) = tokio::join!(Stream::connect(&path), listener.accept());
        let (mut client, mut server) = (client.unwrap(), server.unwrap());
        client.write_all(b"request").await.unwrap();
        let mut request = [0; 7];
        server.read_exact(&mut request).await.unwrap();
        assert_eq!(&request, b"request");
        server.write_all(b"reply").await.unwrap();
        server.shutdown().await.unwrap();
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).await.unwrap();
        assert_eq!(reply, b"reply");
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), exchange)
        .await
        .unwrap();
    drop(listener);
    std::fs::remove_file(&path).unwrap();
    std::fs::write(&path, b"ordinary file").unwrap();
    assert!(Listener::bind(&path).is_err());
    assert!(Stream::connect(&path).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"ordinary file");
}
