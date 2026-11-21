use std::net::IpAddr;

use crate::model_work::loopback::{
    Protocol, Socket, Verdict, is_loopback, listens, read_lsof, read_proc_net, verdict,
};

fn ip(text: &str) -> IpAddr {
    text.parse().expect("an address")
}

#[test]
fn only_the_three_loopback_forms_count() {
    for good in ["127.0.0.1", "::1", "::ffff:127.0.0.1"] {
        assert!(is_loopback(ip(good)), "{good}");
    }
    for bad in [
        "0.0.0.0",
        "::",
        "127.0.0.2",
        "::ffff:127.0.0.2",
        "192.168.1.4",
        "::ffff:0.0.0.0",
    ] {
        assert!(!is_loopback(ip(bad)), "{bad}");
    }
}

#[test]
fn a_socket_on_each_address_is_exposed() {
    let good = Socket {
        protocol: Protocol::Tcp,
        address: ip("127.0.0.1"),
        port: 41000,
    };
    let bad = Socket {
        protocol: Protocol::Udp,
        address: ip("0.0.0.0"),
        port: 5353,
    };
    assert_eq!(verdict(&[good]), Verdict::Loopback);
    assert_eq!(verdict(&[good, bad]), Verdict::Exposed(bad));
    assert!(listens(&[good], 41000));
    assert!(!listens(&[good], 41001));
}

#[test]
fn the_linux_table_reads() {
    let tcp = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 0100007F:A028 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4242 1\n   1: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 4343 1\n   2: 0100007F:A029 0100007F:0050 01 00000000:00000000 00:00000000 00000000  1000        0 4444 1\n";
    let read = read_proc_net(tcp, Protocol::Tcp);
    assert_eq!(read.len(), 2);
    assert_eq!(
        read[0],
        (
            Socket {
                protocol: Protocol::Tcp,
                address: ip("127.0.0.1"),
                port: 41000
            },
            4242
        )
    );
    assert_eq!(read[1].0.address, ip("0.0.0.0"));
    let tcp6 = "head\n   0: 00000000000000000000000001000000:A028 00000000000000000000000000000000:0000 0A 0:0 0:0 0 1000 0 77 1\n   1: 0000000000000000FFFF00000100007F:A02A 00000000000000000000000000000000:0000 0A 0:0 0:0 0 1000 0 78 1\n";
    let read = read_proc_net(tcp6, Protocol::Tcp);
    assert_eq!(read[0].0.address, ip("::1"));
    assert_eq!(read[1].0.address, ip("::ffff:127.0.0.1"));
}

#[test]
fn the_lsof_output_reads() {
    let text = "p123\nf5\nPTCP\nn127.0.0.1:41000\nTST=LISTEN\nf6\nPTCP\nn127.0.0.1:41001->127.0.0.1:80\nTST=ESTABLISHED\nf7\nPUDP\nn*:5353\nf8\nPTCP\nn[::1]:41002\nTST=LISTEN\n";
    let read = read_lsof(text);
    assert_eq!(read.len(), 3);
    assert_eq!(
        read[0],
        Socket {
            protocol: Protocol::Tcp,
            address: ip("127.0.0.1"),
            port: 41000
        }
    );
    assert_eq!(read[1].address, ip("0.0.0.0"));
    assert_eq!(read[2].address, ip("::1"));
    assert!(matches!(verdict(&read), Verdict::Exposed(_)));
}
