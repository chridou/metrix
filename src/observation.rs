use std::time::{Duration, Instant};

use crate::snapshot::ItemKind;

#[derive(Debug, Clone, Copy)]
pub enum TimeUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
}

impl Default for TimeUnit {
    fn default() -> Self {
        TimeUnit::Microseconds
    }
}

/// An observation that has been made.
///
/// Be aware that not all instruments handle all
/// observations or values.
/// E.g. a `Meter` does not take the `value` of
/// an `Observation::ObservedOneValue` into account but
/// simply counts the observation as one occurrence.
#[derive(Debug)]
pub enum Observation<L> {
    /// Observed many occurrences with no value at the given timestamp
    Observed {
        label: L,
        count: u64,
        timestamp: Instant,
    },
    /// Observed one occurrence without a value at the given timestamp
    ObservedOne { label: L, timestamp: Instant },
    /// Observed one occurrence with a value at a given timestamp.
    ObservedOneValue {
        label: L,
        value: ObservedValue,
        timestamp: Instant,
    },
}

impl<L> Observation<L> {
    pub fn observed(label: L, count: u64, timestamp: Instant) -> Self {
        Observation::Observed {
            label,
            count,
            timestamp,
        }
    }

    pub fn observed_now(label: L, count: u64) -> Self {
        Self::observed(label, count, Instant::now())
    }

    pub fn observed_one(label: L, timestamp: Instant) -> Self {
        Observation::ObservedOne { label, timestamp }
    }

    pub fn observed_one_now(label: L) -> Self {
        Self::observed_one(label, Instant::now())
    }

    pub fn observed_one_value<T: Into<ObservedValue>>(
        label: L,
        value: T,
        timestamp: Instant,
    ) -> Self {
        Observation::ObservedOneValue {
            label,
            value: value.into(),
            timestamp,
        }
    }

    pub fn observed_one_value_now<T: Into<ObservedValue>>(label: L, value: T) -> Self {
        Self::observed_one_value(label, value, Instant::now())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ObservedValue {
    SignedInteger(i64),
    UnsignedInteger(u64),
    Float(f64),
    Bool(bool),
    Duration(u64, TimeUnit),
    ChangedBy(i64),
}

impl ObservedValue {
    pub fn to_item_kind(&self) -> Option<ItemKind> {
        match self {
            Self::SignedInteger(v) => Some((*v).into()),
            Self::UnsignedInteger(v) => Some((*v).into()),
            Self::Float(v) => Some((*v).into()),
            Self::Bool(v) => Some((*v).into()),
            Self::Duration(_, _) => None,
            Self::ChangedBy(_) => None,
        }
    }
}

impl From<i64> for ObservedValue {
    fn from(v: i64) -> Self {
        ObservedValue::SignedInteger(v)
    }
}

impl From<i32> for ObservedValue {
    fn from(v: i32) -> Self {
        ObservedValue::SignedInteger(v.into())
    }
}

impl From<i16> for ObservedValue {
    fn from(v: i16) -> Self {
        ObservedValue::SignedInteger(v.into())
    }
}

impl From<i8> for ObservedValue {
    fn from(v: i8) -> Self {
        ObservedValue::SignedInteger(v.into())
    }
}

impl From<u64> for ObservedValue {
    fn from(v: u64) -> Self {
        ObservedValue::UnsignedInteger(v)
    }
}

impl From<u32> for ObservedValue {
    fn from(v: u32) -> Self {
        ObservedValue::UnsignedInteger(v.into())
    }
}

impl From<u16> for ObservedValue {
    fn from(v: u16) -> Self {
        ObservedValue::UnsignedInteger(v.into())
    }
}

impl From<u8> for ObservedValue {
    fn from(v: u8) -> Self {
        ObservedValue::UnsignedInteger(v.into())
    }
}

impl From<usize> for ObservedValue {
    fn from(v: usize) -> Self {
        ObservedValue::UnsignedInteger(v as u64)
    }
}

impl From<f64> for ObservedValue {
    fn from(v: f64) -> Self {
        ObservedValue::Float(v)
    }
}

impl From<bool> for ObservedValue {
    fn from(v: bool) -> Self {
        ObservedValue::Bool(v)
    }
}

impl From<Duration> for ObservedValue {
    fn from(v: Duration) -> Self {
        let nanos = (v.as_secs() * 1_000_000_000) + (v.subsec_nanos() as u64);
        ObservedValue::Duration(nanos, TimeUnit::Nanoseconds)
    }
}

impl From<(u64, TimeUnit)> for ObservedValue {
    fn from(d: (u64, TimeUnit)) -> Self {
        ObservedValue::Duration(d.0, d.1)
    }
}

impl From<(Duration, TimeUnit)> for ObservedValue {
    fn from(d: (Duration, TimeUnit)) -> Self {
        match d.1 {
            TimeUnit::Nanoseconds => {
                let nanos = (d.0.as_secs() * 1_000_000_000) + (d.0.subsec_nanos() as u64);
                (nanos, d.1).into()
            }
            TimeUnit::Microseconds => {
                let micros = (d.0.as_secs() * 1_000_000) + (d.0.subsec_nanos() as u64 / 1_000);

                (micros, d.1).into()
            }
            TimeUnit::Milliseconds => {
                let millis = (d.0.as_secs() * 1_000) + (d.0.subsec_nanos() as u64 / 1_000_000);
                (millis, d.1).into()
            }
            TimeUnit::Seconds => (d.0.as_secs(), d.1).into(),
        }
    }
}

impl From<super::Increment> for ObservedValue {
    fn from(_: super::Increment) -> Self {
        ObservedValue::ChangedBy(1)
    }
}

impl From<super::Decrement> for ObservedValue {
    fn from(_: super::Decrement) -> Self {
        ObservedValue::ChangedBy(-1)
    }
}

impl From<super::IncrementBy> for ObservedValue {
    fn from(v: super::IncrementBy) -> Self {
        ObservedValue::ChangedBy(v.0.into())
    }
}

impl From<super::DecrementBy> for ObservedValue {
    fn from(v: super::DecrementBy) -> Self {
        let v: i64 = v.0.into();
        ObservedValue::ChangedBy(-v)
    }
}

impl From<super::ChangeBy> for ObservedValue {
    fn from(v: super::ChangeBy) -> Self {
        ObservedValue::ChangedBy(v.0)
    }
}

impl ObservedValue {
    pub fn convert_to_i64(&self) -> Option<i64> {
        match *self {
            ObservedValue::SignedInteger(v) => Some(v),
            ObservedValue::UnsignedInteger(v) => {
                if v <= std::i64::MAX as u64 {
                    Some(v as i64)
                } else {
                    None
                }
            }
            ObservedValue::Float(v) => {
                if v <= std::i64::MAX as f64 {
                    Some(v.round() as i64)
                } else {
                    None
                }
            }

            ObservedValue::Bool(_) => None,
            ObservedValue::Duration(_, _) => None,
            ObservedValue::ChangedBy(_) => None,
        }
    }

    pub fn convert_to_u64(&self) -> Option<u64> {
        match *self {
            ObservedValue::SignedInteger(v) => {
                if v >= 0 {
                    Some(v as u64)
                } else {
                    None
                }
            }
            ObservedValue::UnsignedInteger(v) => Some(v),
            ObservedValue::Float(v) => {
                if v <= std::u64::MAX as f64 {
                    Some(v.round() as u64)
                } else {
                    None
                }
            }

            ObservedValue::Bool(_) => None,
            ObservedValue::Duration(_, _) => None,
            ObservedValue::ChangedBy(_) => None,
        }
    }

    pub fn convert_to_bool(&self) -> Option<bool> {
        match *self {
            ObservedValue::SignedInteger(v) => Some(v != 0),
            ObservedValue::UnsignedInteger(v) => Some(v != 0),
            ObservedValue::Float(_) => None,
            ObservedValue::Bool(v) => Some(v),
            ObservedValue::Duration(_, _) => None,
            ObservedValue::ChangedBy(_) => None,
        }
    }
}

impl<L> Observation<L> {
    /// Extracts the label `L` from an observation.
    pub fn label(&self) -> &L {
        match *self {
            Observation::Observed { ref label, .. } => label,
            Observation::ObservedOne { ref label, .. } => label,
            Observation::ObservedOneValue { ref label, .. } => label,
        }
    }
}

impl<L> Observation<L> {
    pub fn timestamp(&self) -> Instant {
        match *self {
            Observation::Observed { timestamp, .. } => timestamp,
            Observation::ObservedOne { timestamp, .. } => timestamp,
            Observation::ObservedOneValue { timestamp, .. } => timestamp,
        }
    }
}

pub trait ObservationLike {
    fn timestamp(&self) -> Instant;
}

impl<L> ObservationLike for Observation<L> {
    fn timestamp(&self) -> Instant {
        match *self {
            Observation::Observed { timestamp, .. } => timestamp,
            Observation::ObservedOne { timestamp, .. } => timestamp,
            Observation::ObservedOneValue { timestamp, .. } => timestamp,
        }
    }
}
