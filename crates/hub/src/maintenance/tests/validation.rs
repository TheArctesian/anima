//! Checking a window typed in the form (MW-001, MW-003, MW-004).

use super::*;
use crate::maintenance::{
    FieldError, TargetType, WindowKind,
    model::{Target, TargetForm, WindowForm},
    rules::validate,
};

pub(super) const DEVICE: &str = "0190a000-0000-7000-8002-000000000001";

pub(super) fn target(kind: TargetType, text: &str) -> TargetForm {
    TargetForm {
        kind,
        target: text.into(),
    }
}

/// A valid manual window; each test changes what it checks.
pub(super) fn form(kind: WindowKind) -> WindowForm {
    WindowForm {
        name: "Patch night".into(),
        kind,
        timezone: "Pacific/Auckland".into(),
        starts_at: None,
        ends_at: None,
        first_start_at: None,
        every_days: None,
        weekdays: None,
        days_of_month: None,
        start_time: None,
        duration_minutes: None,
        targets: vec![target(TargetType::Tag, "infra")],
    }
}

pub(super) fn weekday_form() -> WindowForm {
    WindowForm {
        weekdays: Some(vec![Weekday::Sun]),
        start_time: Some("03:00".into()),
        duration_minutes: Some(60),
        ..form(WindowKind::RecurringWeekday)
    }
}

pub(super) fn day_form() -> WindowForm {
    WindowForm {
        days_of_month: Some(vec![1, -1]),
        start_time: Some("03:00".into()),
        duration_minutes: Some(60),
        ..form(WindowKind::RecurringDayOfMonth)
    }
}

pub(super) fn interval_form() -> WindowForm {
    WindowForm {
        first_start_at: Some(at("2026-11-01T00:00:00Z")),
        every_days: Some(7),
        duration_minutes: Some(60),
        ..form(WindowKind::RecurringInterval)
    }
}

pub(super) fn single_form() -> WindowForm {
    WindowForm {
        starts_at: Some(at("2026-11-04T09:00:00Z")),
        ends_at: Some(at("2026-11-04T10:00:00Z")),
        ..form(WindowKind::Single)
    }
}

pub(super) fn fields(form: &WindowForm) -> Vec<&'static str> {
    validate(form)
        .unwrap_err()
        .iter()
        .map(|error| error.field)
        .collect()
}

#[test]
fn mw_004_every_kind_with_its_own_fields_is_valid() {
    for form in [
        form(WindowKind::Manual),
        single_form(),
        interval_form(),
        weekday_form(),
        day_form(),
    ] {
        let kind = form.kind;
        let checked = validate(&form).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        assert_eq!(checked.rule.kind(), kind);
    }
}

#[test]
fn mw_004_the_checked_weekday_rule_holds_the_fields_as_typed() {
    let checked = validate(&weekday_form()).unwrap();
    assert_eq!(checked.rule, weekly(&[Weekday::Sun], "03:00", 60));
    assert_eq!(checked.timezone, AUCKLAND);
    assert_eq!(checked.targets, vec![Target::Tag("infra".into())]);
}

#[test]
fn mw_004_a_field_of_another_kind_is_refused_and_a_missing_field_is_reported() {
    let weekday_with_single_fields = WindowForm {
        starts_at: Some(at("2026-11-04T09:00:00Z")),
        every_days: Some(2),
        ..weekday_form()
    };
    assert_eq!(
        fields(&weekday_with_single_fields),
        ["startsAt", "everyDays"]
    );

    let manual_with_duration = WindowForm {
        duration_minutes: Some(30),
        ..form(WindowKind::Manual)
    };
    assert_eq!(fields(&manual_with_duration), ["durationMinutes"]);

    let day_without_days = WindowForm {
        days_of_month: None,
        ..day_form()
    };
    assert_eq!(fields(&day_without_days), ["daysOfMonth"]);

    let single_without_end = WindowForm {
        ends_at: None,
        ..single_form()
    };
    assert_eq!(fields(&single_without_end), ["endsAt"]);
}

#[test]
fn mw_004_start_time_is_two_digit_hours_and_minutes() {
    for bad in [
        "3:00", "03:0", "0300", "24:00", "12:60", "03:00:00", "ab:cd", " 3:00", "+3:00", "",
    ] {
        let form = WindowForm {
            start_time: Some(bad.into()),
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["startTime"], "{bad:?}");
    }
    for good in ["00:00", "23:59", "09:05"] {
        let form = WindowForm {
            start_time: Some(good.into()),
            ..day_form()
        };
        assert!(validate(&form).is_ok(), "{good}");
    }
}

#[test]
fn mw_002_duration_is_between_one_minute_and_seven_days() {
    for bad in [0, -1, 10_081, i32::MAX, i32::MIN] {
        let form = WindowForm {
            duration_minutes: Some(bad),
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["durationMinutes"], "{bad}");
    }
    for good in [1, 10_080] {
        let form = WindowForm {
            duration_minutes: Some(good),
            ..interval_form()
        };
        assert!(validate(&form).is_ok(), "{good}");
    }
}

#[test]
fn mw_004_every_days_is_at_least_one() {
    for bad in [0, -3] {
        let form = WindowForm {
            every_days: Some(bad),
            ..interval_form()
        };
        assert_eq!(fields(&form), ["everyDays"], "{bad}");
    }
    let one = WindowForm {
        every_days: Some(1),
        ..interval_form()
    };
    assert!(validate(&one).is_ok());
}

#[test]
fn mw_004_a_single_window_must_end_after_it_starts() {
    for ends_at in ["2026-11-04T09:00:00Z", "2026-11-04T08:00:00Z"] {
        let form = WindowForm {
            ends_at: Some(at(ends_at)),
            ..single_form()
        };
        assert_eq!(fields(&form), ["endsAt"], "{ends_at}");
    }
}

#[test]
fn mw_004_weekdays_are_not_empty_and_not_repeated() {
    for bad in [vec![], vec![Weekday::Mon, Weekday::Tue, Weekday::Mon]] {
        let form = WindowForm {
            weekdays: Some(bad),
            ..weekday_form()
        };
        assert_eq!(fields(&form), ["weekdays"]);
    }
}

#[test]
fn mw_004_days_of_month_are_one_to_31_or_minus_one_without_repeats() {
    for bad in [
        vec![],
        vec![0],
        vec![32],
        vec![-2],
        vec![5, 5],
        vec![i32::MAX],
        vec![1, -1, 1],
    ] {
        let form = WindowForm {
            days_of_month: Some(bad.clone()),
            ..day_form()
        };
        assert_eq!(fields(&form), ["daysOfMonth"], "{bad:?}");
    }
    let edge = WindowForm {
        days_of_month: Some(vec![1, 31, -1]),
        ..day_form()
    };
    assert!(validate(&edge).is_ok());
}

#[test]
fn mw_004_every_invalid_field_is_reported_together() {
    let form = WindowForm {
        name: String::new(),
        timezone: "nowhere".into(),
        start_time: Some("25:00".into()),
        duration_minutes: Some(0),
        weekdays: Some(vec![]),
        targets: vec![],
        ..weekday_form()
    };
    let errors: Vec<FieldError> = validate(&form).unwrap_err();
    assert_eq!(
        errors.iter().map(|e| e.field).collect::<Vec<_>>(),
        [
            "name",
            "timezone",
            "weekdays",
            "startTime",
            "durationMinutes",
            "targets"
        ]
    );
}

fn refused_fast(form: &WindowForm, field: &str) {
    let started = std::time::Instant::now();
    let errors = validate(form).unwrap_err();
    assert!(
        started.elapsed() < std::time::Duration::from_millis(500),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].field, field);
}

#[test]
fn api_005_an_oversized_days_of_month_list_is_refused_at_once() {
    let form = WindowForm {
        days_of_month: Some((1..=150_000).collect()),
        ..day_form()
    };
    refused_fast(&form, "daysOfMonth");
}

#[test]
fn api_005_an_oversized_weekdays_list_is_refused_at_once() {
    let form = WindowForm {
        weekdays: Some(vec![Weekday::Sun; 150_000]),
        ..weekday_form()
    };
    refused_fast(&form, "weekdays");
}

#[test]
fn mw_004_a_repeated_day_of_month_or_weekday_is_still_refused() {
    let days = WindowForm {
        days_of_month: Some(vec![1, 15, 1]),
        ..day_form()
    };
    refused_fast(&days, "daysOfMonth");
    let week = WindowForm {
        weekdays: Some(vec![Weekday::Mon, Weekday::Mon]),
        ..weekday_form()
    };
    refused_fast(&week, "weekdays");
}
