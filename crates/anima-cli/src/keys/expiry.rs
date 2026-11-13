//! When an API key stops working: a time, or a number of days from now. Every key expires and
//! none lasts longer than a year (INT-003, D-23).

use chrono::{DateTime, Duration, SecondsFormat, Utc};

/// The most days a key may last. One day more than a year, so a key made on 29 February can
/// still last until the same date next year; the hub decides the exact limit.
const MAX_DAYS: i64 = 366;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ExpiryError {
    #[error("--expires is a number of days such as 90d, or a time such as 2027-01-31T00:00:00Z")]
    NotAWhen,
    #[error("--expires is in the past")]
    Past,
    #[error("--expires is more than a year away; an API key lasts at most one year")]
    TooLong,
}

/// `text` as an RFC 3339 time in UTC, counted from `now` when it is `<days>d`.
///
/// # Errors
/// [`ExpiryError`] for text that is no time, a time that has passed or one over a year away.
pub fn parse(text: &str, now: DateTime<Utc>) -> Result<String, ExpiryError> {
    let text = text.trim();
    let at = match text.strip_suffix('d') {
        Some(days) => {
            let days: i64 = days.parse().map_err(|_| ExpiryError::NotAWhen)?;
            if days < 1 {
                return Err(ExpiryError::Past);
            }
            now.checked_add_signed(Duration::days(days.min(MAX_DAYS + 1)))
                .ok_or(ExpiryError::TooLong)?
        }
        None => DateTime::parse_from_rfc3339(text)
            .map_err(|_| ExpiryError::NotAWhen)?
            .with_timezone(&Utc),
    };
    if at <= now {
        return Err(ExpiryError::Past);
    }
    if at > now + Duration::days(MAX_DAYS) {
        return Err(ExpiryError::TooLong);
    }
    Ok(at.to_rfc3339_opts(SecondsFormat::Secs, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-13T10:00:00Z")
            .expect("a time")
            .with_timezone(&Utc)
    }

    #[test]
    fn int_003_days_count_from_now_and_a_time_is_kept() {
        assert_eq!(parse("90d", now()), Ok("2027-02-11T10:00:00Z".to_owned()));
        assert_eq!(
            parse("2027-01-01T12:00:00+02:00", now()),
            Ok("2027-01-01T10:00:00Z".to_owned())
        );
    }

    #[test]
    fn int_003_a_key_cannot_outlast_a_year_or_expire_already() {
        assert_eq!(parse("400d", now()), Err(ExpiryError::TooLong));
        assert_eq!(parse("99999999999999d", now()), Err(ExpiryError::TooLong));
        assert_eq!(
            parse("2028-01-01T00:00:00Z", now()),
            Err(ExpiryError::TooLong)
        );
        assert_eq!(parse("0d", now()), Err(ExpiryError::Past));
        assert_eq!(parse("2026-11-01T00:00:00Z", now()), Err(ExpiryError::Past));
        assert_eq!(parse("soon", now()), Err(ExpiryError::NotAWhen));
        assert_eq!(parse("365d", now()).map(|_| ()), Ok(()));
    }
}
