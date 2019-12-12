//! Instruments that track values and/or derive values
//! from observations.
use std::time::Instant;

use crate::{Observation, ObservedValue, PutsSnapshot, TimeUnit};

pub use self::counter::Counter;
pub use self::gauge::*;
pub use self::histogram::Histogram;
pub use self::instrument_adapter::*;
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

pub(crate) enum LabelFilter<L> {
    AcceptNone,
    AcceptAll,
    One(L),
    Two(L, L),
    Three(L, L, L),
    Four(L, L, L, L),
    Five(L, L, L, L, L),
    Many(Vec<L>),
    Predicate(Box<dyn Fn(&L) -> bool + Send + 'static>),
}

impl<L> LabelFilter<L>
where
    L: PartialEq + Eq,
{
    pub fn new(label: L) -> Self {
        Self::One(label)
    }

    pub fn predicate<P>(p: P) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self::Predicate(Box::new(p))
    }

    pub fn many(mut labels: Vec<L>) -> Self {
        if labels.is_empty() {
            return LabelFilter::AcceptNone;
        }

        if labels.len() == 1 {
            return LabelFilter::One(labels.pop().unwrap());
        }

        if labels.len() == 2 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            return LabelFilter::Two(b, a);
        }

        if labels.len() == 3 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            return LabelFilter::Three(c, b, a);
        }

        if labels.len() == 4 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            return LabelFilter::Four(d, c, b, a);
        }

        if labels.len() == 5 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            let ee = labels.pop().unwrap();
            return LabelFilter::Five(ee, d, c, b, a);
        }

        LabelFilter::Many(labels)
    }

    pub fn accepts(&self, label: &L) -> bool {
        match self {
            LabelFilter::AcceptNone => false,
            LabelFilter::AcceptAll => true,
            LabelFilter::One(a) => label == a,
            LabelFilter::Two(a, b) => label == a || label == b,
            LabelFilter::Three(a, b, c) => label == a || label == b || label == c,
            LabelFilter::Four(a, b, c, d) => label == a || label == b || label == c || label == d,
            LabelFilter::Five(a, b, c, d, ee) => {
                label == a || label == b || label == c || label == d || label == ee
            }
            LabelFilter::Many(many) => many.contains(label),
            LabelFilter::Predicate(ref pred) => pred(label),
        }
    }
}

impl<L> Default for LabelFilter<L> {
    fn default() -> Self {
        Self::AcceptAll
    }
}

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
mod test_label_filter {
    use super::*;

    #[test]
    fn empty_filter() {
        let filter = LabelFilter::AcceptNone;
        assert!(!filter.accepts(&1));
    }

    #[test]
    fn accept_all_filter() {
        let filter = LabelFilter::AcceptAll;
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
    }

    #[test]
    fn accept_one_filter() {
        let filter = LabelFilter::One(1);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(!filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_two_filter() {
        let filter = LabelFilter::Two(1, 2);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_three_filter() {
        let filter = LabelFilter::Three(1, 2, 3);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_four_filter() {
        let filter = LabelFilter::Four(1, 2, 3, 4);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_five_filter() {
        let filter = LabelFilter::Five(1, 2, 3, 4, 5);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_many_filter() {
        let filter = LabelFilter::Many(vec![1, 2, 3, 4, 5]);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn many_filters() {
        let max = 20;
        for n in 1..max {
            let mut labels = Vec::new();
            for i in 1..=n {
                labels.push(i);
            }

            let filter = LabelFilter::many(labels);

            assert!(!filter.accepts(&0));
            assert!(!filter.accepts(&max));

            for i in 1..=n {
                assert!(filter.accepts(&i));
            }
        }
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
