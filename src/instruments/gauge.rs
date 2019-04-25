use std::time::{Duration, Instant};

use instruments::{Instrument, Update, Updates};
use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot};

/// Simply returns the value that has been observed last.
///
/// Reacts to the following `Observation`:
///
/// * `Obervation::ObservedOneValue`(Update::ObservationWithValue)
///
/// # Example
///
/// ```
/// use std::time::Instant;
/// use metrix::instruments::*;
///
/// let mut gauge = Gauge::new_with_defaults("example");
/// assert_eq!(None, gauge.get());
/// let update = Update::ObservationWithValue(12, Instant::now());
/// gauge.update(&update);
///
/// assert_eq!(Some(12), gauge.get());
/// ```
pub struct Gauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<(u64, u64)>,
    peak_keep_alive: Option<Duration>,
    last_peak_at: Instant,
}

impl Gauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
            peak_keep_alive: None,
            last_peak_at: Instant::now(),
        }
    }

    pub fn set(&mut self, v: u64) {
        let new_value = if let Some((_, peak)) = self.value.take() {
            if let Some(peak_dur) = self.peak_keep_alive {
                let now = Instant::now();
                if v > peak {
                    self.last_peak_at = now;
                    Some((v, v))
                } else if self.last_peak_at > now - peak_dur {
                    Some((v, peak))
                } else {
                    Some((v, v))
                }
            } else {
                Some((v, v))
            }
        } else {
            self.last_peak_at = Instant::now();
            Some((v, v))
        };
        self.value = new_value;
    }

    pub fn get(&self) -> Option<u64> {
        self.value.as_ref().map(|v| v.0)
    }

    /// If set to `Some(Duration)` a peak value will
    /// be diplayed for the given duration unless there
    /// is a new peak. The field has the name `[gauge_name]_peak`.
    ///
    /// If set to None the peak values will not be shown.
    pub fn set_peak_keep_alive(&mut self, d: Duration) {
        self.peak_keep_alive = Some(d)
    }

    /// Gets the name of this `Gauge`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `Gauge`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `Gauge`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `Gauge`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }
}

impl Instrument for Gauge {}

impl PutsSnapshot for Gauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        if let Some((value, peak)) = self.value {
            into.items.push((self.name.clone(), value.into()));
            if let Some(peak_dur) = self.peak_keep_alive {
                let peak_name = format!("{}_peak", self.name);
                if self.last_peak_at > Instant::now() - peak_dur {
                    into.items.push((peak_name, peak.into()));
                } else {
                    into.items.push((peak_name, value.into()));
                }
            }
        }
    }
}

impl Updates for Gauge {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(v, _) => {
                self.set(v);
                1
            }
            _ => 0,
        }
    }
}

impl Descriptive for Gauge {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
