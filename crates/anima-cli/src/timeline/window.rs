//! The span of time `anima timeline list` asks for: the day before now, unless the operator
//! gives its start or its end. Pure.

use chrono::{DateTime, Duration, SecondsFormat, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WindowError {
    #[error("--from is a time such as 2026-11-01T00:00:00Z")]
    From,
    #[error("--to is a time such as 2026-11-02T00:00:00Z")]
    To,
    #[error("--to must be after --from")]
    Empty,
}

fn instant(text: &str) -> Option<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(text.trim()).ok()?;
    Some(parsed.with_timezone(&Utc))
}

/// The start and the end to send, as RFC 3339 in UTC. With no `to` the span ends at `now`;
/// with no `from` it starts one day before its end.
///
/// # Errors
/// [`WindowError`] for text that is no time, or an end that is not after the start.
pub fn window(
    from: Option<&str>,
    to: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(String, String), WindowError> {
    let to = match to {
        Some(text) => instant(text).ok_or(WindowError::To)?,
        None => now,
    };
    let from = match from {
        Some(text) => instant(text).ok_or(WindowError::From)?,
        None => to - Duration::days(1),
    };
    if from >= to {
        return Err(WindowError::Empty);
    }
    let text = |at: DateTime<Utc>| at.to_rfc3339_opts(SecondsFormat::Micros, true);
    Ok((text(from), text(to)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap()
    }

    #[test]
    fn tml_007_with_no_time_the_span_is_the_day_before_now() {
        assert_eq!(
            window(None, None, now()),
            Ok((
                "2026-11-15T12:00:00.000000Z".to_owned(),
                "2026-11-16T12:00:00.000000Z".to_owned()
            ))
        );
    }

    #[test]
    fn tml_007_a_time_in_another_zone_is_the_same_instant_in_utc() {
        let span = window(Some("2026-11-16T08:00:00+08:00"), None, now()).unwrap();
        assert_eq!(span.0, "2026-11-16T00:00:00.000000Z");
        let day = window(None, Some("2026-11-10T00:00:00Z"), now()).unwrap();
        assert_eq!(day.0, "2026-11-09T00:00:00.000000Z");
    }

    #[test]
    fn tml_007_text_that_is_no_time_or_an_empty_span_is_refused() {
        assert_eq!(
            window(Some("yesterday"), None, now()),
            Err(WindowError::From)
        );
        assert_eq!(
            window(None, Some("2026-11-16"), now()),
            Err(WindowError::To)
        );
        assert_eq!(
            window(Some("2026-11-16T12:00:00Z"), None, now()),
            Err(WindowError::Empty)
        );
    }
}
