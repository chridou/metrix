//! Switched have a boolean state that is changed by
//! `Observation`s.
//!
//! Switches can be used to attach alerts.

mod staircase_timer;
mod occurrence_indicator;
mod non_occurrence_indicator;

pub use self::staircase_timer::StaircaseTimer;
pub use self::occurrence_indicator::OccurrenceIndicator;
pub use self::non_occurrence_indicator::NonOccurrenceIndicator;
