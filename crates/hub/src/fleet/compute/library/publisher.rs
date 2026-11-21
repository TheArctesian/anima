//! How the publisher of a base model gets onto the approved list (LIBM-003, R-NEW-modellibrary).
//! Pure rules: the hub setting and the actor come in as values.
#![expect(
    clippy::unused_async_trait_impl,
    reason = "the async-graphql Enum derive generates async trait impls"
)]

use async_graphql::Enum;

use super::naming;

/// The hub setting. The operator chose `BaseModel` (R-NEW-modellibrary).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Enum)]
#[graphql(name = "ModelPublisherApproval")]
pub enum PublisherApproval {
    /// When a person adds a model from an approved repository, the organisation of its base
    /// model joins the approved list.
    #[default]
    BaseModel,
    /// Only the operator adds an organisation, by hand.
    ByHand,
}

impl PublisherApproval {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BaseModel => "base_model",
            Self::ByHand => "by_hand",
        }
    }

    #[must_use]
    pub fn from_stored(text: &str) -> Option<Self> {
        [Self::BaseModel, Self::ByHand]
            .into_iter()
            .find(|mode| mode.as_str() == text)
    }
}

/// What an add does to the approved list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Widening {
    /// No publisher, it is in the list already, or the setting is `ByHand`.
    Nothing,
    /// The publisher joins the list.
    Add(String),
    /// The publisher stays out: an API key or an agent added the model. Only a person widens
    /// the list (R-NEW-modellibrary).
    Withheld(String),
}

/// What adding a model with this `publisher` does to the `approved` list. `person` is true
/// for a person at a login or browser token, false for an API key or an agent.
#[must_use]
pub fn widening(
    mode: PublisherApproval,
    person: bool,
    publisher: Option<&str>,
    approved: &[String],
) -> Widening {
    let Some(publisher) = publisher else {
        return Widening::Nothing;
    };
    if mode == PublisherApproval::ByHand || naming::is_approved(publisher, approved) {
        Widening::Nothing
    } else if person {
        Widening::Add(publisher.to_owned())
    } else {
        Widening::Withheld(publisher.to_owned())
    }
}
