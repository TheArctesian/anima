//! The socket of the interface of the proxy (SF-NEW-revroutesocket): mode `0660`, and an answer
//! only to a peer with the user ID of the proxy.

use std::{os::unix::fs::MetadataExt as _, os::unix::fs::PermissionsExt as _, sync::Arc};

use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::UnixStream,
};

use super::door::{FakeConnector, door};
use crate::work::routes::door::{bind, serve};

/// Asks for the table on the socket at `path` and gives the whole answer.
async fn ask_table(path: &std::path::Path) -> String {
    let mut stream = UnixStream::connect(path).await.unwrap();
    stream.write_all(b"\"table\"\n").await.ok();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).await.unwrap_or_default();
    answer
}

db_test! {
    async fn the_socket_answers_only_the_user_of_the_proxy(pool: PgPool) {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("routes.sock");
        let listener = bind(&path).unwrap();
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o660);
        let own = metadata.uid();

        let mut other = door(&pool, Arc::new(FakeConnector::default()), 4);
        other.proxy_uid = own.wrapping_add(1);
        let serving = tokio::spawn(serve(listener, Arc::new(other)));
        assert_eq!(ask_table(&path).await, "", "a different user gets no answer");
        serving.abort();

        let listener = bind(&path).unwrap();
        let mut proxy = door(&pool, Arc::new(FakeConnector::default()), 4);
        proxy.proxy_uid = own;
        let serving = tokio::spawn(serve(listener, Arc::new(proxy)));
        assert_eq!(ask_table(&path).await.trim(), r#"{"names":[]}"#);
        serving.abort();
    }
}
