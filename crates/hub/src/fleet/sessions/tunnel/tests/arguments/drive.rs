//! The shared drive of an RDP device is its own folder under the drive root (RA-021).

use super::{
    Desktop, PASSWORD, Secret, Sources, TARGET, connect_by_name_of, connect_for, connect_values,
    device,
};

#[test]
fn ra_021_each_device_gets_its_own_folder_under_the_drive_root() {
    let mut other = device("desk.example.net");
    other.id = "0190a000-0000-7000-8002-0000000000bb".parse().unwrap();
    for device in [device("desk.example.net"), other] {
        let values = connect_for(Desktop::Rdp, &device);
        assert_eq!(
            values["drive-path"],
            format!("/var/lib/guacd/drives/{}", device.id)
        );
    }
}

#[test]
fn ra_021_a_root_with_a_trailing_slash_gets_no_double_slash() {
    let device = device("desk");
    let password = Secret::from(PASSWORD.to_owned());
    let sources = Sources {
        device: &device,
        target: TARGET,
        port: Desktop::Rdp.port(&device),
        password: Some(&password),
        certificate: None,
        drive_root: "/srv/drives/",
    };
    let values = connect_values(Desktop::Rdp, &sources).unwrap();
    let values = connect_by_name_of(Desktop::Rdp, &device, values);
    assert_eq!(values["drive-path"], format!("/srv/drives/{}", device.id));
    assert_eq!(values["cert-fingerprints"], "", "no certificate, no pin");
}
