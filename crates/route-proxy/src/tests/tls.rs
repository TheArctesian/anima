//! A certificate serves only the name it is valid for.

use zeroize::Zeroizing;

use crate::{
    tls::server_config,
    wire::{Named, Table},
};

fn named(name: &str, certified_for: &str) -> Named {
    let key = rcgen::KeyPair::generate().unwrap();
    let params = rcgen::CertificateParams::new(vec![certified_for.to_owned()]).unwrap();
    let certificate = params.self_signed(&key).unwrap();
    Named {
        name: name.to_owned(),
        application: uuid::Uuid::nil(),
        chain_pem: certificate.pem(),
        key_pem: Zeroizing::new(key.serialize_pem()),
    }
}

#[test]
fn a_certificate_of_a_different_name_or_a_broken_key_is_left_out() {
    let mut broken = named("broken.example.com", "broken.example.com");
    broken.key_pem = Zeroizing::new("not a key".to_owned());
    let table = Table {
        names: vec![
            named("photos.example.com", "photos.example.com"),
            named("hub.example.com", "photos.example.com"),
            broken,
        ],
        challenges: Vec::new(),
    };
    let (_, left_out) = server_config(&table).unwrap();
    assert_eq!(left_out, ["hub.example.com", "broken.example.com"]);
}
