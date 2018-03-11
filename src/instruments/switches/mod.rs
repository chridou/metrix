//! Switched have a boolean state that is changed by
//! `Observation`s.
//!
//! Switches can be used to attach alerts.

mod staircase_timer;
mod occurance_indicator;
mod non_occurance_indicator;

pub use self::staircase_timer::*;
pub use self::occurance_indicator::*;
pub use self::non_occurance_indicator::*;
