//! GraphQL scalars that more than one domain uses (K1, R-012).
//!
//! `DateTime` is `chrono::DateTime<Utc>` and `JSON` is `async_graphql::Json<T>`; both come from
//! async-graphql. This module adds the network scalars, so the merged schema registers each name once.

use std::{fmt, net, str::FromStr};

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use ipnet::IpNet;

/// An IPv4 or IPv6 address, written as `10.10.0.12` or `fd00::1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IpAddr(pub net::IpAddr);

/// A network in CIDR form with no host bits set, written as `10.10.0.0/24`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cidr(IpNet);

/// The text is not a network, or it has host bits set (`10.10.0.5/24`).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CidrError {
    #[error("not a network in CIDR form, for example 10.10.0.0/24")]
    Syntax,
    #[error("host bits are set; use {0}")]
    HostBits(IpNet),
}

impl Cidr {
    /// Accepts only a network address: `10.10.0.0/24`, not `10.10.0.5/24`.
    ///
    /// # Errors
    ///
    /// [`CidrError::HostBits`] names the network the caller probably meant.
    pub fn new(net: IpNet) -> Result<Self, CidrError> {
        let network = net.trunc();
        if network == net {
            Ok(Self(net))
        } else {
            Err(CidrError::HostBits(network))
        }
    }

    #[must_use]
    pub fn net(self) -> IpNet {
        self.0
    }
}

impl FromStr for Cidr {
    type Err = CidrError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text.parse().map_err(|_| CidrError::Syntax)?)
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Parses a GraphQL string value; the error text is safe to show to the operator.
fn parse_string<T: FromStr>(value: &Value, what: &str) -> Result<T, String>
where
    T::Err: fmt::Display,
{
    match value {
        Value::String(text) => text.parse().map_err(|error: T::Err| error.to_string()),
        _ => Err(format!("{what} must be a string")),
    }
}

#[Scalar(name = "IpAddr")]
impl ScalarType for IpAddr {
    fn parse(value: Value) -> InputValueResult<Self> {
        parse_string::<net::IpAddr>(&value, "IpAddr")
            .map(Self)
            .map_err(InputValueError::custom)
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_string())
    }
}

#[Scalar(name = "Cidr")]
impl ScalarType for Cidr {
    fn parse(value: Value) -> InputValueResult<Self> {
        parse_string(&value, "Cidr").map_err(InputValueError::custom)
    }

    fn to_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vpn_001_cidr_accepts_a_network_and_rejects_host_bits() {
        assert_eq!(
            "10.10.0.0/24".parse::<Cidr>().map(|c| c.to_string()),
            Ok("10.10.0.0/24".to_owned())
        );
        assert_eq!(
            "10.10.0.5/24".parse::<Cidr>(),
            Err(CidrError::HostBits(
                "10.10.0.0/24".parse().expect("valid network")
            ))
        );
        assert_eq!("10.10.0.0".parse::<Cidr>(), Err(CidrError::Syntax));
    }

    #[test]
    fn scalars_parse_strings_and_refuse_other_values() {
        let ip =
            <IpAddr as ScalarType>::parse(Value::String("fd00::1".into())).expect("valid address");
        assert_eq!(ip.to_value(), Value::String("fd00::1".into()));
        assert!(<IpAddr as ScalarType>::parse(Value::String("10.10.0.256".into())).is_err());
        assert!(<IpAddr as ScalarType>::parse(Value::Number(1.into())).is_err());
        assert!(<Cidr as ScalarType>::parse(Value::String("10.30.0.0/24".into())).is_ok());
    }
}
