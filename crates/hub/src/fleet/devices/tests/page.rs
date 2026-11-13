use super::*;

#[test]
fn api_006_cursor_round_trips_any_name() {
    for name in ["pve-01", "", "docker host", "ünï-ζ", "a.b/c"] {
        assert_eq!(decode_cursor(&encode_cursor(name)).unwrap(), name);
    }
    assert_ne!(encode_cursor("pve-01"), "pve-01");
}

#[test]
fn api_006_bad_cursor_is_an_after_field_error() {
    for bad in [
        "",
        "pve-01",
        "n1.zz",
        "n1.abc",
        "n1.ff",
        "n2.7076652d3031",
        "n1.é",
    ] {
        let error = decode_cursor(bad).unwrap_err();
        assert_eq!(error.field, "after", "{bad:?}");
    }
}

#[test]
fn api_006_first_must_be_1_to_200() {
    assert_eq!(page_size(1).unwrap(), 1);
    assert_eq!(page_size(200).unwrap(), 200);
    for bad in [0, -5, 201, i32::MAX, i32::MIN] {
        assert_eq!(page_size(bad).unwrap_err().field, "first", "{bad}");
    }
}

#[test]
fn api_006_split_page_reports_whether_more_rows_follow() {
    let more = split_page(vec![1, 2, 3], 2);
    assert_eq!((more.rows, more.has_next_page), (vec![1, 2], true));
    let exact = split_page(vec![1, 2], 2);
    assert_eq!((exact.rows, exact.has_next_page), (vec![1, 2], false));
    let none = split_page(Vec::<i32>::new(), 2);
    assert_eq!((none.rows, none.has_next_page), (vec![], false));
}

#[test]
fn api_006_in_memory_pages_of_two_cover_every_device_once_in_name_order() {
    let names = [
        "docker-host",
        "home-assistant",
        "ollama",
        "pihole",
        "pve-01",
    ];
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    let mut pages = 0;
    loop {
        let page = page_after(&names, |name| name, after.as_deref(), 2);
        pages += 1;
        seen.extend(page.rows.iter().map(|name| (*name).to_owned()));
        if !page.has_next_page {
            break;
        }
        let last = page.rows.last().unwrap();
        after = Some(decode_cursor(&encode_cursor(last)).unwrap());
    }
    assert_eq!(seen, names);
    assert_eq!(pages, 3);
}

#[test]
fn api_006_in_memory_page_after_the_last_name_is_empty() {
    let names = ["a", "b"];
    let page = page_after(&names, |name| name, Some("b"), 5);
    assert!(page.rows.is_empty() && !page.has_next_page);
    let from_gap = page_after(&names, |name| name, Some("aa"), 5);
    assert_eq!(from_gap.rows, [&"b"]);
}

#[test]
fn api_006_in_memory_paging_follows_byte_order_of_names() {
    let names = ["Zeta", "alpha", "émile"];
    let first = page_after(&names, |name| name, None, 1);
    assert_eq!(first.rows, [&"Zeta"]);
    let second = page_after(&names, |name| name, Some("Zeta"), 1);
    assert_eq!(second.rows, [&"alpha"]);
    assert!(second.has_next_page);
    let third = page_after(&names, |name| name, Some("alpha"), 5);
    assert_eq!(third.rows, [&"émile"]);
    assert!(!third.has_next_page);
}
