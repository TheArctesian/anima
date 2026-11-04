//! The HTTP probe ignores proxy variables in the environment.

use super::support::{Reply, env, serve, target};
use crate::checks::{
    config::{CheckConfig, HttpConfig},
    probe::probe,
};

async fn run(config: HttpConfig) -> crate::checks::probe::ProbeResult {
    probe(&target("unused"), &CheckConfig::Http(config), None, &env()).await
}

const PROXY_CHILD: &str = "ANIMA_TEST_PROXY_CHILD";

/// Runs only inside the child process of `proxy_variables_in_the_environment_are_ignored`.
#[tokio::test]
async fn proxy_environment_child() {
    if std::env::var_os(PROXY_CHILD).is_none() {
        return;
    }
    let server = serve(None, |_| Reply::ok("direct")).await;
    let result = run(HttpConfig::new(server.url("/"))).await;
    assert!(result.ok, "{:?}", result.message);
    assert_eq!(server.seen().len(), 1);
}

#[test]
fn proxy_variables_in_the_environment_are_ignored() {
    let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    proxy.set_nonblocking(true).unwrap();
    let url = format!("http://{}", proxy.local_addr().unwrap());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "checks::tests::probe::proxy::proxy_environment_child",
            "--exact",
            "--test-threads=1",
        ])
        .env(PROXY_CHILD, "1")
        .env("HTTP_PROXY", &url)
        .env("http_proxy", &url)
        .env("ALL_PROXY", &url)
        .env("all_proxy", &url)
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .status()
        .unwrap();
    assert!(
        status.success(),
        "the child probe went through the proxy or failed"
    );
    assert!(proxy.accept().is_err(), "the proxy received a connection");
}
