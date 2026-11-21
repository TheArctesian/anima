//! Tests of the parse of the power source of a Mac and of a Linux laptop.

use super::*;

#[test]
fn cap_004_the_power_source_of_a_mac_and_of_a_linux_laptop() {
    let mains = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t93%; charging; 0:45 remaining present: true\n";
    let battery =
        "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t37%; discharging\n";
    assert_eq!(from_pmset(mains), Power::Mains);
    assert_eq!(from_pmset(battery), Power::Battery { percent: 37 });
    let supply = |scope: &str| Supply {
        kind: "Battery".into(),
        scope: scope.into(),
        status: "Discharging".into(),
        capacity: Some(41),
    };
    assert_eq!(
        from_supplies(&[supply("System")]),
        Power::Battery { percent: 41 }
    );
    assert_eq!(from_supplies(&[supply("Device")]), Power::Mains, "a mouse");
}
