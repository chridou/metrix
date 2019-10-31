//! Switched have a boolean state that is changed by
//! `Observation`s.
//!
//! Switches can be used to attach alerts.
use std::borrow::Cow;

mod flag;
mod non_occurrence_indicator;
mod occurrence_indicator;
mod staircase_timer;

pub use self::flag::Flag;
pub use self::non_occurrence_indicator::NonOccurrenceIndicator;
pub use self::occurrence_indicator::OccurrenceIndicator;
pub use self::staircase_timer::StaircaseTimer;

/// Describes how to change a name using the given `String` in the variant.
#[derive(Debug, Clone)]
pub enum NameAlternation {
    /// Prefix the original name
    Prefix(String),
    /// Postfix the original name
    Postfix(String),
    /// Replace the original name
    Rename(String),
}

impl NameAlternation {
    fn adjust_name<'a>(&'a self, original: &str) -> Cow<'a, str> {
        match self {
            NameAlternation::Rename(s) => Cow::Borrowed(s),
            NameAlternation::Prefix(s) => Cow::Owned(format!("{}{}", s, original)),
            NameAlternation::Postfix(s) => Cow::Owned(format!("{}{}", original, s)),
        }
    }
}
