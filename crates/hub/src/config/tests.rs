//! Parsing of every variable from a map, without the process environment (K7).

use super::*;

pub(super) fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn empty_env_gives_the_documented_defaults() {
    let config = from_env(&env(&[])).unwrap();
    assert_eq!(config.listen, "127.0.0.1:8443".parse().unwrap());
    assert_eq!(
        config.database_url_file,
        PathBuf::from("/run/secrets/database_url")
    );
    assert_eq!(config.key_file, PathBuf::from("/run/secrets/anima_key"));
    assert_eq!(config.guacd, "guacd:4822");
    assert_eq!(
        config.permitted_networks,
        vec!["100.64.0.0/10".parse().unwrap()]
    );
    assert_eq!(
        config.guacd_drive_root,
        PathBuf::from("/var/lib/guacd/drives")
    );
    assert_eq!(config.tailscale_localapi, None);
    assert_eq!(config.log, "info");
    assert_eq!(config.tls_cert_file, None);
    assert_eq!(config.tls_key_file, None);
    assert_eq!(config.probe_deny, vec![]);
}

#[test]
fn r_030_probe_deny_reads_comma_separated_cidrs_and_empty_means_none() {
    let text = "172.30.0.0/24, 172.30.1.0/24,203.0.113.7/32,fd00::/8";
    let config = from_env(&env(&[(PROBE_DENY, text)])).unwrap();
    let expected: Vec<ipnet::IpNet> = text
        .split(',')
        .map(|net| net.trim().parse().unwrap())
        .collect();
    assert_eq!(config.probe_deny, expected);
    let empty = from_env(&env(&[(PROBE_DENY, "")])).unwrap();
    assert_eq!(empty.probe_deny, vec![]);
}

#[test]
fn r_030_probe_deny_rejects_empty_entries_and_bad_cidrs_naming_the_variable() {
    for bad in [
        "172.30.0.0/24,",
        ",172.30.0.0/24",
        "10.0.0.0/8,,10.1.0.0/16",
    ] {
        let error = from_env(&env(&[(PROBE_DENY, bad)])).unwrap_err();
        let expected = matches!(
            error,
            Error::NetworkEmpty {
                variable: PROBE_DENY
            }
        );
        assert!(expected, "{bad}");
    }
    let error = from_env(&env(&[(PROBE_DENY, "172.30.0.0/33")])).unwrap_err();
    assert!(matches!(
        error,
        Error::Network {
            variable: PROBE_DENY,
            ..
        }
    ));
    let text = error.to_string();
    assert!(
        text.starts_with("ANIMA_PROBE_DENY entry \"172.30.0.0/33\""),
        "{text}"
    );
    let error = from_env(&env(&[(PERMITTED_NETWORKS, "nope")])).unwrap_err();
    let text = error.to_string();
    assert!(text.starts_with("ANIMA_PERMITTED_NETWORKS entry"), "{text}");
}

#[test]
fn every_variable_overrides_its_default() {
    let config = from_env(&env(&[
        (LISTEN, "10.0.0.1:9000"),
        (DATABASE_URL_FILE, "/db"),
        (KEY_FILE, "/key"),
        (GUACD, "gw.internal:4000"),
        (PERMITTED_NETWORKS, "10.0.0.0/8, 192.168.1.0/24"),
        (LOG, "debug"),
        (TLS_CERT_FILE, "/cert.pem"),
        (TLS_KEY_FILE, "/key.pem"),
    ]))
    .unwrap();
    assert_eq!(config.listen, "10.0.0.1:9000".parse().unwrap());
    assert_eq!(config.database_url_file, PathBuf::from("/db"));
    assert_eq!(config.key_file, PathBuf::from("/key"));
    assert_eq!(config.guacd, "gw.internal:4000");
    assert_eq!(config.permitted_networks.len(), 2);
    assert_eq!(config.log, "debug");
    assert_eq!(config.tls_cert_file, Some(PathBuf::from("/cert.pem")));
    assert_eq!(config.tls_key_file, Some(PathBuf::from("/key.pem")));
}

#[test]
fn invalid_listen_names_the_variable() {
    let error = from_env(&env(&[(LISTEN, "nope")])).unwrap_err();
    assert!(matches!(error, Error::Listen { .. }));
    assert!(error.to_string().contains(LISTEN));
}

#[test]
fn guacd_accepts_bracketed_ipv6_and_rejects_unbracketed() {
    let config = from_env(&env(&[(GUACD, "[::1]:4822")])).unwrap();
    assert_eq!(config.guacd, "[::1]:4822");
    let error = from_env(&env(&[(GUACD, "::1:4822")])).unwrap_err();
    assert!(matches!(
        error,
        Error::HostPort {
            variable: GUACD,
            ..
        }
    ));
}

#[test]
fn empty_tls_files_count_as_unset() {
    let config = from_env(&env(&[(TLS_CERT_FILE, ""), (TLS_KEY_FILE, "")])).unwrap();
    assert_eq!((config.tls_cert_file, config.tls_key_file), (None, None));
}

#[test]
fn collect_env_rejects_non_unicode_anima_and_skips_other_non_unicode() {
    use std::os::unix::ffi::OsStringExt;
    let bad = || OsString::from_vec(vec![0xff, 0xfe]);
    let good = [(OsString::from("ANIMA_LOG"), OsString::from("debug"))];

    let skipped = [
        (OsString::from("OTHER"), bad()),
        (bad(), OsString::from("x")),
    ];
    let map = collect_env(good.clone().into_iter().chain(skipped)).unwrap();
    assert_eq!(map, env(&[(LOG, "debug")]));

    let value = [(OsString::from("ANIMA_LOG"), bad())];
    let error = collect_env(value).unwrap_err();
    assert!(matches!(error, Error::NotUnicode { name } if name == "ANIMA_LOG"));

    let name = [(
        OsString::from_vec(b"ANIMA_\xff".to_vec()),
        OsString::from("x"),
    )];
    assert!(matches!(collect_env(name), Err(Error::NotUnicode { .. })));
}

#[test]
fn guacd_without_host_or_valid_port_is_rejected() {
    for bad in ["guacd", ":4822", "guacd:", "guacd:70000", "guacd:http"] {
        let error = from_env(&env(&[(GUACD, bad)])).unwrap_err();
        assert!(
            matches!(
                error,
                Error::HostPort {
                    variable: GUACD,
                    ..
                }
            ),
            "{bad}"
        );
    }
}

#[test]
fn permitted_networks_reject_empty_entries_and_bad_cidrs() {
    for bad in ["10.0.0.0/8,", ",10.0.0.0/8", "10.0.0.0/8,,192.168.0.0/16"] {
        let error = from_env(&env(&[(PERMITTED_NETWORKS, bad)])).unwrap_err();
        assert!(matches!(error, Error::NetworkEmpty { .. }), "{bad}");
    }
    let error = from_env(&env(&[(PERMITTED_NETWORKS, "10.0.0.0/99")])).unwrap_err();
    assert!(matches!(error, Error::Network { .. }));
}

#[test]
fn empty_value_is_rejected_instead_of_defaulted() {
    let error = from_env(&env(&[(LOG, "")])).unwrap_err();
    assert!(matches!(error, Error::Empty { variable: LOG }));
}

#[test]
fn tls_cert_without_key_is_rejected_and_so_is_key_without_cert() {
    let error = from_env(&env(&[(TLS_CERT_FILE, "/c")])).unwrap_err();
    assert!(matches!(
        error,
        Error::TlsIncomplete {
            missing: TLS_KEY_FILE,
            ..
        }
    ));
    let error = from_env(&env(&[(TLS_KEY_FILE, "/k")])).unwrap_err();
    assert!(matches!(
        error,
        Error::TlsIncomplete {
            missing: TLS_CERT_FILE,
            ..
        }
    ));
}

#[test]
fn ra_021_guacd_drive_root_overrides_default_and_must_be_absolute() {
    let config = from_env(&env(&[(GUACD_DRIVE_ROOT, "/srv/drives")])).unwrap();
    assert_eq!(config.guacd_drive_root, PathBuf::from("/srv/drives"));
    let error = from_env(&env(&[(GUACD_DRIVE_ROOT, "")])).unwrap_err();
    assert!(matches!(
        error,
        Error::Empty {
            variable: GUACD_DRIVE_ROOT
        }
    ));
    let error = from_env(&env(&[(GUACD_DRIVE_ROOT, "drives/here")])).unwrap_err();
    assert!(matches!(
        error,
        Error::NotAbsolute {
            variable: GUACD_DRIVE_ROOT,
            ..
        }
    ));
    assert!(error.to_string().contains(GUACD_DRIVE_ROOT));
}

#[test]
fn d_4_tailscale_localapi_parses_host_port_and_empty_means_none() {
    for good in ["proxy:41112", "[fd7a::1]:41112"] {
        let config = from_env(&env(&[(TAILSCALE_LOCALAPI, good)])).unwrap();
        assert_eq!(config.tailscale_localapi.as_deref(), Some(good));
    }
    let config = from_env(&env(&[(TAILSCALE_LOCALAPI, "")])).unwrap();
    assert_eq!(config.tailscale_localapi, None);
}

#[test]
fn d_4_malformed_tailscale_localapi_is_refused_naming_the_variable() {
    for bad in ["proxy", ":1", "proxy:x", "fd7a::1:41112"] {
        let error = from_env(&env(&[(TAILSCALE_LOCALAPI, bad)])).unwrap_err();
        assert!(
            matches!(
                error,
                Error::HostPort {
                    variable: TAILSCALE_LOCALAPI,
                    ..
                }
            ),
            "{bad}"
        );
        assert!(error.to_string().contains(TAILSCALE_LOCALAPI), "{bad}");
    }
}

#[test]
fn k7_phase_3_variables_default_when_unset() {
    let config = from_env(&env(&[])).unwrap();
    assert_eq!(config.public_origin, "https://hub.anima.ing");
    assert_eq!(config.web_root, None);
    assert_eq!(config.recording_root, None);
    assert_eq!(config.recording_max_bytes, 2_147_483_648);
}

#[test]
fn k7_phase_3_variables_take_explicit_values() {
    let config = from_env(&env(&[
        (PUBLIC_ORIGIN, "https://anima.example.org:8443"),
        (WEB_ROOT, "/usr/share/anima/web"),
        (RECORDING_ROOT, "/var/lib/anima/recordings"),
        (RECORDING_MAX_BYTES, "1048576"),
    ]))
    .unwrap();
    assert_eq!(config.public_origin, "https://anima.example.org:8443");
    assert_eq!(config.web_root, Some(PathBuf::from("/usr/share/anima/web")));
    assert_eq!(
        config.recording_root,
        Some(PathBuf::from("/var/lib/anima/recordings"))
    );
    assert_eq!(config.recording_max_bytes, 1_048_576);
}

#[test]
fn d_2_empty_web_root_and_empty_recording_root_are_none() {
    let config = from_env(&env(&[(WEB_ROOT, ""), (RECORDING_ROOT, "")])).unwrap();
    assert_eq!(config.web_root, None);
    assert_eq!(config.recording_root, None);
}

#[test]
fn k7_recording_max_bytes_must_be_a_positive_integer_naming_the_variable() {
    for bad in ["0", "-5", "+5", "lots", "1.5", "99999999999999999999999"] {
        let error = from_env(&env(&[(RECORDING_MAX_BYTES, bad)])).unwrap_err();
        assert!(
            matches!(error, Error::NotPositive { variable, .. } if variable == RECORDING_MAX_BYTES),
            "{bad:?}"
        );
    }
    let error = from_env(&env(&[(RECORDING_MAX_BYTES, "")])).unwrap_err();
    assert!(matches!(error, Error::Empty { variable } if variable == RECORDING_MAX_BYTES));
    let minimum = from_env(&env(&[(RECORDING_MAX_BYTES, "1")])).unwrap();
    assert_eq!(minimum.recording_max_bytes, 1);
}

#[test]
fn d_2_web_root_and_recording_root_must_be_absolute_and_not_the_file_system_root() {
    for variable in [WEB_ROOT, RECORDING_ROOT] {
        for relative in ["web", "./web", " "] {
            let error = from_env(&env(&[(variable, relative)])).unwrap_err();
            assert!(
                matches!(error, Error::NotAbsolute { variable: named, .. } if named == variable),
                "{variable} {relative:?}"
            );
        }
        let error = from_env(&env(&[(variable, "/")])).unwrap_err();
        assert!(matches!(error, Error::FilesystemRoot { variable: named } if named == variable));
    }
}

#[test]
fn d_2_roots_with_a_dot_dot_part_are_refused_naming_the_variable() {
    for variable in [WEB_ROOT, RECORDING_ROOT] {
        for bad in ["/..", "/usr/..", "/srv/../web"] {
            let error = from_env(&env(&[(variable, bad)])).unwrap_err();
            assert!(
                matches!(error, Error::ParentDirectory { variable: named } if named == variable),
                "{variable} {bad:?}"
            );
        }
        let config = from_env(&env(&[(variable, "/srv/web")])).unwrap();
        assert!(config.web_root.or(config.recording_root).is_some());
    }
}
