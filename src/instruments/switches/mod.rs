//! Switched have a boolean state that is changed by
//! `Observation`s.
//!
//! Switches can be used to attach alerts.

mod flag;
mod non_occurrence_indicator;
mod occurrence_indicator;
mod staircase_timer;

pub use self::flag::Flag;
pub use self::non_occurrence_indicator::NonOccurrenceIndicator;
pub use self::occurrence_indicator::OccurrenceIndicator;
pub use self::staircase_timer::StaircaseTimer;
