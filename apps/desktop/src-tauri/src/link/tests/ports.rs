//! The open ports of this computer (HMD-049) as the shell asks `anima-link ports`, with a
//! stand-in link program.

use std::fs;

use super::super::open_ports::{OpenPorts, ask, may_open, open_ports};
use super::runs::stand_in;

/// `anima-link ports` as the shell asks it: the list is the file `open` beside the program.
/// Each call is noted.
const PORTS_LINK: &str = r#"
here=$(dirname "$0")
echo "$*" >> "$here/calls"
[ "$1 $2" = "ports -state" ] || exit 9
touch "$here/open"
case "$4" in
on) grep -qx "$5" "$here/open" || echo "$5" >> "$here/open" ;;
off) grep -vx "$5" "$here/open" > "$here/open.next"; mv "$here/open.next" "$here/open" ;;
"") ;;
*) exit 9 ;;
esac
printf '{"open":[%s]}\n' "$(sort -n "$here/open" | paste -sd, -)"
"#;

#[test]
fn hmd_049_the_person_turns_a_port_on_and_off_and_the_list_is_read_back() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), PORTS_LINK);
    let device = folder.path().join("device");

    assert_eq!(
        ask(&program, &device, None).unwrap(),
        OpenPorts { open: vec![] }
    );
    assert_eq!(
        ask(&program, &device, Some((5900, true))).unwrap().open,
        [5900]
    );
    assert_eq!(
        ask(&program, &device, Some((22, true))).unwrap().open,
        [22, 5900]
    );
    assert_eq!(
        ask(&program, &device, Some((5900, false))).unwrap().open,
        [22]
    );

    let calls = fs::read_to_string(folder.path().join("calls")).unwrap();
    let state = device.display();
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        [
            format!("ports -state {state}"),
            format!("ports -state {state} on 5900"),
            format!("ports -state {state} on 22"),
            format!("ports -state {state} off 5900"),
        ]
    );
}

#[test]
fn hmd_049_a_port_that_the_link_may_not_open_is_not_asked_for() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), PORTS_LINK);
    let device = folder.path().join("device");
    for port in [80, 8080, 3389] {
        assert!(
            ask(&program, &device, Some((port, true))).is_err(),
            "{port}"
        );
    }
    assert!(
        !folder.path().join("calls").exists(),
        "the link program was not run"
    );
}

#[test]
fn the_answer_of_the_link_program_is_read_strictly() {
    assert_eq!(
        open_ports(br#"{"open":[22,5900]}"#, false),
        Some(OpenPorts {
            open: vec![22, 5900]
        })
    );
    assert_eq!(
        open_ports(br#"{"open":[3389]}"#, true).map(|p| p.open),
        Some(vec![3389])
    );
    for printed in [
        &br#"{"open":[3389]}"#[..],
        br#"{"open":[80]}"#,
        br#"{"open":"22"}"#,
        b"not json",
        b"",
    ] {
        assert_eq!(
            open_ports(printed, false),
            None,
            "{}",
            String::from_utf8_lossy(printed)
        );
    }
    assert!(may_open(22, false) && may_open(5900, false));
    assert!(!may_open(3389, false) && may_open(3389, true));
}

#[test]
fn a_link_program_that_fails_gives_an_error() {
    let folder = tempfile::tempdir().unwrap();
    let program = stand_in(folder.path(), "exit 1");
    assert!(ask(&program, &folder.path().join("device"), None).is_err());
}
