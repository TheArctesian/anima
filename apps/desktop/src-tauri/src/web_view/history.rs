//! The toolbar's history steps (WEB-005): back, forward and reload. Pure: no I/O.

use super::WebViewError;

/// A history step or a reload, as the client names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Back,
    Forward,
    Reload,
}

impl Step {
    /// Reads `BACK`, `FORWARD` or `RELOAD`; nothing else.
    pub fn parse(text: &str) -> Result<Self, WebViewError> {
        match text {
            "BACK" => Ok(Self::Back),
            "FORWARD" => Ok(Self::Forward),
            "RELOAD" => Ok(Self::Reload),
            _ => Err(WebViewError::Step),
        }
    }

    /// The fixed script that takes the step in the page, or `None` for a reload, which the
    /// web view does itself. No part of a script comes from input.
    pub fn script(self) -> Option<&'static str> {
        match self {
            Self::Back => Some("history.back()"),
            Self::Forward => Some("history.forward()"),
            Self::Reload => None,
        }
    }
}
