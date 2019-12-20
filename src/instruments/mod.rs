//! Instruments that track values and/or derive values
//! from observations.
use std::time::Instant;

use crate::{Observation, ObservedValue, PutsSnapshot, TimeUnit};

pub use self::counter::Counter;
pub use self::gauge::*;
pub use self::histogram::Histogram;
pub use self::instrument_adapter::*;
pub use self::label_filter::*;
pub use self::meter::Meter;
pub use self::other_instruments::*;
pub use self::panel::*;
pub use self::polled::*;
pub use self::switches::*;
pub use crate::cockpit::Cockpit;

mod counter;
mod fundamentals;
mod gauge;
mod histogram;
mod instrument_adapter;
#[cfg(feature = "jemalloc-ctl")]
pub mod jemalloc;
mod label_filter;
mod meter;
pub mod other_instruments;
mod panel;
pub mod polled;
pub mod switches;

#[derive(Debug, Clone)]
/// An update instruction for an instrument
pub enum Update {
    /// Many observations without a value observed at a given time
    Observations(u64, Instant),
    /// One observation without a value observed at a given time
    Observation(Instant),
    /// One observation with a value observed at a given time
    ObservationWithValue(ObservedValue, Instant),
}

/// A label with the associated `Update`
///
/// This is basically a split `Observation`
pub struct LabelAndUpdate<T>(pub T, pub Update);

impl<T> From<Observation<T>> for LabelAndUpdate<T> {
    fn from(obs: Observation<T>) -> LabelAndUpdate<T> {
        match obs {
            Observation::Observed {
                label,
                count,
                timestamp,
                ..
            } => LabelAndUpdate(label, Update::Observations(count, timestamp)),
            Observation::ObservedOne {
                label, timestamp, ..
            } => LabelAndUpdate(label, Update::Observation(timestamp)),
            Observation::ObservedOneValue {
                label,
                value,
                timestamp,
                ..
            } => LabelAndUpdate(label, Update::ObservationWithValue(value, timestamp)),
        }
    }
}

/// A label with the associated `Update`
///
/// This is basically a split `Observation`
pub struct BorrowedLabelAndUpdate<'a, T: 'a>(pub &'a T, pub Update);

impl<'a, T> From<&'a Observation<T>> for BorrowedLabelAndUpdate<'a, T> {
    fn from(obs: &'a Observation<T>) -> BorrowedLabelAndUpdate<'a, T> {
        match obs {
            Observation::Observed {
                label,
                count,
                timestamp,
                ..
            } => BorrowedLabelAndUpdate(label, Update::Observations(*count, *timestamp)),
            Observation::ObservedOne {
                label, timestamp, ..
            } => BorrowedLabelAndUpdate(label, Update::Observation(*timestamp)),
            Observation::ObservedOneValue {
                label,
                value,
                timestamp,
                ..
            } => BorrowedLabelAndUpdate(label, Update::ObservationWithValue(*value, *timestamp)),
        }
    }
}

/// Implementors of `Updates`
/// can handle `Update`s.
///
/// `Update`s are basically observations without a label.
pub trait Updates {
    /// Update the internal state according to the given `Update`.
    ///
    /// Not all `Update`s might modify the internal state.
    /// Only those that are appropriate and meaningful for
    /// the implementor.
    ///
    /// Returns the number of instruments updated
    fn update(&mut self, with: &Update) -> usize;
}

/// Requirement for an instrument
pub trait Instrument: Updates + PutsSnapshot {}

fn duration_to_display_value(time: u64, current_unit: TimeUnit, target_unit: TimeUnit) -> u64 {
    use TimeUnit::*;
    match (current_unit, target_unit) {
        (Nanoseconds, Nanoseconds) => time,
        (Nanoseconds, Microseconds) => time / 1_000,
        (Nanoseconds, Milliseconds) => time / 1_000_000,
        (Nanoseconds, Seconds) => time / 1_000_000_000,
        (Microseconds, Nanoseconds) => time * 1_000,
        (Microseconds, Microseconds) => time,
        (Microseconds, Milliseconds) => time / 1_000,
        (Microseconds, Seconds) => time / 1_000_000,
        (Milliseconds, Nanoseconds) => time * 1_000_000,
        (Milliseconds, Microseconds) => time * 1_000,
        (Milliseconds, Milliseconds) => time,
        (Milliseconds, Seconds) => time / 1_000,
        (Seconds, Nanoseconds) => time * 1_000_000_000,
        (Seconds, Microseconds) => time * 1_000_000,
        (Seconds, Milliseconds) => time * 1_000,
        (Seconds, Seconds) => time,
    }
}

#[cfg(test)]
mod test_time_conversion {
    use super::duration_to_display_value;
    use crate::TimeUnit;

    #[test]
    fn duration_to_display_value_from_nanos() {
        let nanos = 1_234_567_890;
        assert_eq!(
            duration_to_display_value(nanos, TimeUnit::Nanoseconds, TimeUnit::Nanoseconds),
            1_234_567_890,
            "to nanos"
        );
        assert_eq!(
            duration_to_display_value(nanos, TimeUnit::Nanoseconds, TimeUnit::Microseconds),
            1_234_567,
            "to micros"
        );
        assert_eq!(
            duration_to_display_value(nanos, TimeUnit::Nanoseconds, TimeUnit::Milliseconds),
            1_234,
            "to millis"
        );
        assert_eq!(
            duration_to_display_value(nanos, TimeUnit::Nanoseconds, TimeUnit::Seconds),
            1,
            "to seconds"
        );
    }

    #[test]
    fn duration_to_display_value_from_micros() {
        let micros = 1_234_567;
        assert_eq!(
            duration_to_display_value(micros, TimeUnit::Microseconds, TimeUnit::Nanoseconds),
            1_234_567_000,
            "to nanos"
        );
        assert_eq!(
            duration_to_display_value(micros, TimeUnit::Microseconds, TimeUnit::Microseconds),
            1_234_567,
            "to micros"
        );
        assert_eq!(
            duration_to_display_value(micros, TimeUnit::Microseconds, TimeUnit::Milliseconds),
            1_234,
            "to millis"
        );
        assert_eq!(
            duration_to_display_value(micros, TimeUnit::Microseconds, TimeUnit::Seconds),
            1,
            "to seconds"
        );
    }

    #[test]
    fn duration_to_display_value_from_millis() {
        let millis = 1_234;
        assert_eq!(
            duration_to_display_value(millis, TimeUnit::Milliseconds, TimeUnit::Nanoseconds),
            1_234_000_000,
            "to nanos"
        );
        assert_eq!(
            duration_to_display_value(millis, TimeUnit::Milliseconds, TimeUnit::Microseconds),
            1_234_000,
            "to micros"
        );
        assert_eq!(
            duration_to_display_value(millis, TimeUnit::Milliseconds, TimeUnit::Milliseconds),
            1_234,
            "to millis"
        );
        assert_eq!(
            duration_to_display_value(millis, TimeUnit::Milliseconds, TimeUnit::Seconds),
            1,
            "to seconds"
        );
    }

    #[test]
    fn duration_to_display_value_from_seconds() {
        let seconds = 1;
        assert_eq!(
            duration_to_display_value(seconds, TimeUnit::Seconds, TimeUnit::Nanoseconds),
            1_000_000_000,
            "to nanos"
        );
        assert_eq!(
            duration_to_display_value(seconds, TimeUnit::Seconds, TimeUnit::Microseconds),
            1_000_000,
            "to micros"
        );
        assert_eq!(
            duration_to_display_value(seconds, TimeUnit::Seconds, TimeUnit::Milliseconds),
            1_000,
            "to millis"
        );
        assert_eq!(
            duration_to_display_value(seconds, TimeUnit::Seconds, TimeUnit::Seconds),
            1,
            "to seconds"
        );
    }
}
