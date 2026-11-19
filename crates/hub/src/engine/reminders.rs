//! Reminders: the evaluation of a rule, as a pure function (K-E6 Sections 1 to 4; REM-001,
//! REM-003, REM-004, REM-006). An extension gives occurrences and never changes this code
//! (ENG-001). The store, the sweep and the delivery are later tasks of E0-7.

mod evaluation;
mod model;
mod place_state;
mod schedule;
#[cfg(test)]
mod tests;

pub use evaluation::{day_start, evaluate};
pub use model::{
    Class, Comparison, Crossing, Due, History, Known, Limits, Moment, Occurrence, PlaceChange,
    Rule, Stop, TimeTrigger, Trigger, Unless,
};
pub use place_state::{PlaceState, place_changes};
