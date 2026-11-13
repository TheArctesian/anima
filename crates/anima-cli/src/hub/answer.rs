//! Reads a GraphQL answer: the first error decides the outcome; otherwise `data` must have the
//! shape the document asked for. The HTTP status matters only when the body is not GraphQL.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::error::{Code, FieldError, HubError};

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    #[serde(default)]
    message: String,
    #[serde(default)]
    extensions: Option<Extensions>,
}

#[derive(Deserialize)]
struct Extensions {
    code: Option<String>,
    #[serde(default)]
    fields: Vec<FieldError>,
}

/// The `data` of an answer with no errors, read as `T`.
///
/// # Errors
/// [`HubError::Refused`] for the first GraphQL error; [`HubError::Malformed`] for a body that
/// is not a GraphQL answer of this shape.
pub fn read<T: DeserializeOwned>(status: u16, body: &[u8]) -> Result<T, HubError> {
    let Ok(envelope) = serde_json::from_slice::<Envelope<T>>(body) else {
        return Err(not_graphql(status));
    };
    if let Some(error) = envelope.errors.into_iter().next() {
        return Err(refused(error));
    }
    envelope.data.ok_or(HubError::Malformed { status })
}

/// The whole answer as JSON, for `anima api`, and the first GraphQL error in it, if any.
///
/// # Errors
/// [`HubError::Malformed`] for a body that is not a JSON object.
pub fn read_raw(
    status: u16,
    body: &[u8],
) -> Result<(serde_json::Value, Option<HubError>), HubError> {
    let value: serde_json::Value = serde_json::from_slice(body).map_err(|_| not_graphql(status))?;
    if !value.is_object() {
        return Err(not_graphql(status));
    }
    let envelope = Envelope::<serde::de::IgnoredAny>::deserialize(&value)
        .map_err(|_| HubError::Malformed { status })?;
    let first = envelope.errors.into_iter().next().map(refused);
    Ok((value, first))
}

fn refused(error: GraphqlError) -> HubError {
    let (code, fields) = match error.extensions {
        Some(extensions) => (Code::parse(extensions.code.as_deref()), extensions.fields),
        None => (Code::Internal, Vec::new()),
    };
    HubError::Refused {
        code,
        message: error.message,
        fields,
    }
}

/// A body that is not GraphQL: a refusal in front of the GraphQL layer (NET-002), a proxy's
/// page, or a broken answer. Its status is the only thing to go by.
fn not_graphql(status: u16) -> HubError {
    let code = match status {
        401 => Code::Unauthenticated,
        403 => Code::Forbidden,
        502..=504 => Code::Unavailable,
        _ => return HubError::Malformed { status },
    };
    HubError::Refused {
        code,
        message: format!("The hub refused the request (HTTP status {status})."),
        fields: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Me {
        me: String,
    }

    #[test]
    fn cmd_022_the_first_error_gives_its_code_message_and_fields() {
        let body = br#"{"data":null,"errors":[
            {"message":"Check the name.","extensions":{"code":"VALIDATION","fields":[{"field":"name","message":"Too long."}]}},
            {"message":"second","extensions":{"code":"CONFLICT"}}]}"#;
        let error = read::<Me>(200, body).expect_err("refused");
        assert_eq!(error.code(), Code::Validation);
        assert_eq!(error.to_string(), "Check the name.");
        assert_eq!(error.fields()[0].field, "name");
    }

    #[test]
    fn cmd_021_an_unknown_or_missing_code_is_internal() {
        let body = br#"{"errors":[{"message":"x","extensions":{"code":"TEAPOT"}}]}"#;
        assert_eq!(read::<Me>(200, body).expect_err("x").code(), Code::Internal);
        let body = br#"{"errors":[{"message":"x"}]}"#;
        assert_eq!(read::<Me>(200, body).expect_err("x").code(), Code::Internal);
    }

    #[test]
    fn cmd_003_data_of_the_wrong_shape_or_no_body_is_malformed() {
        assert!(matches!(
            read::<Me>(200, br#"{"data":{"me":3}}"#),
            Err(HubError::Malformed { status: 200 })
        ));
        assert!(matches!(
            read::<Me>(200, b"<html>"),
            Err(HubError::Malformed { .. })
        ));
        assert!(matches!(
            read::<Me>(200, br#"{"data":null}"#),
            Err(HubError::Malformed { .. })
        ));
        assert_eq!(
            read::<Me>(200, br#"{"data":{"me":"op"}}"#).expect("data"),
            Me { me: "op".into() }
        );
    }

    #[test]
    fn cmd_021_a_body_that_is_not_graphql_goes_by_its_status() {
        assert_eq!(
            read::<Me>(403, b"Forbidden").expect_err("x").code(),
            Code::Forbidden
        );
        assert_eq!(
            read::<Me>(503, b"").expect_err("x").code(),
            Code::Unavailable
        );
        assert_eq!(
            read::<Me>(401, b"").expect_err("x").code(),
            Code::Unauthenticated
        );
    }

    /// Every prefix of an answer, and every one-byte change of it, reads to a result: the
    /// reader trusts no length and never panics on a hostile body.
    #[test]
    fn cmd_003_no_cut_or_changed_answer_makes_the_reader_panic() {
        let body = br#"{"data":{"me":"op"},"errors":[{"message":"m","extensions":{"code":"CONFLICT","fields":[{"field":"f","message":"g"}]}}]}"#;
        for end in 0..=body.len() {
            let _ = read::<Me>(200, &body[..end]);
            let _ = read_raw(200, &body[..end]);
        }
        for at in 0..body.len() {
            for byte in [b'"', b'{', b']', 0, 0xff, b'\\', b'9'] {
                let mut changed = body.to_vec();
                changed[at] = byte;
                let _ = read::<Me>(500, &changed);
                let _ = read_raw(500, &changed);
            }
        }
    }

    #[test]
    fn cmd_003_a_raw_answer_keeps_the_body_and_names_the_first_error() {
        let body =
            br#"{"data":null,"errors":[{"message":"No.","extensions":{"code":"FORBIDDEN"}}]}"#;
        let (value, error) = read_raw(200, body).expect("json");
        assert!(value["errors"].is_array());
        assert_eq!(error.map(|error| error.code()), Some(Code::Forbidden));
        assert!(read_raw(200, b"[1]").is_err());
    }
}
