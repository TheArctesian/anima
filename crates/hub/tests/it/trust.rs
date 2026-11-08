//! The trust domain (WP-41 T6; Q2-05, D-31): its GraphQL part and its schema. Accepting host
//! keys and server certificates after a connection test is tested with the connection test,
//! in `sessions::host_keys` and `sessions::server_certificates`.

mod k1;
mod tables;
