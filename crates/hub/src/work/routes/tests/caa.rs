//! The CAA check before a certificate (Section 12.5).

use super::support::{FakeDns, name};
use crate::work::routes::{
    caa::{CaaRefusal, check},
    certificates::AUTHORITY,
    proof::Caa,
};

fn issue(value: &str) -> Caa {
    Caa {
        tag: "issue".to_owned(),
        value: value.to_owned(),
    }
}

#[tokio::test]
async fn no_record_permits_each_authority() {
    let dns = FakeDns::default();
    assert_eq!(
        check(&dns, &name("photos.example.com"), AUTHORITY).await,
        Ok(())
    );
}

#[tokio::test]
async fn the_first_set_from_the_name_up_decides_and_names_its_domain() {
    let mut dns = FakeDns::default();
    dns.caa
        .insert("example.com".to_owned(), vec![issue("pki.goog")]);
    assert_eq!(
        check(&dns, &name("photos.example.com"), AUTHORITY).await,
        Err(CaaRefusal::NotPermitted {
            domain: "example.com".to_owned(),
            authority: AUTHORITY.to_owned(),
        })
    );
    dns.caa.insert(
        "photos.example.com".to_owned(),
        vec![issue("letsencrypt.org; validationmethods=tls-alpn-01")],
    );
    assert_eq!(
        check(&dns, &name("photos.example.com"), AUTHORITY).await,
        Ok(())
    );
}

#[tokio::test]
async fn a_set_with_no_issue_record_permits_and_a_failed_read_refuses() {
    let mut dns = FakeDns::default();
    let iodef = Caa {
        tag: "iodef".to_owned(),
        value: "mailto:security@example.com".to_owned(),
    };
    dns.caa.insert("example.com".to_owned(), vec![iodef]);
    assert_eq!(
        check(&dns, &name("photos.example.com"), AUTHORITY).await,
        Ok(())
    );
    dns.fail("photos.example.com");
    assert_eq!(
        check(&dns, &name("photos.example.com"), AUTHORITY).await,
        Err(CaaRefusal::Unread)
    );
}
