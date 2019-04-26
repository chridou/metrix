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
    value: Option<State>,
    memorize_extrema: Option<Duration>,
}

impl Gauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
            memorize_extrema: None,
        }
    }

    pub fn set(&mut self, v: u64) {
        if let Some(mut state) = self.value.take() {
            if let Some(ext_dur) = self.memorize_extrema {
                let now = Instant::now();
                if v > state.peak {
                    state.last_peak_at = now;
                    state.peak = v;
                } else if state.last_peak_at < now - ext_dur {
                    state.peak = v;
                }

                if v < state.bottom {
                    state.last_bottom_at = now;
                    state.bottom = v;
                } else if state.last_bottom_at < now - ext_dur {
                    state.bottom = v;
                }
            }
            state.current = v;
        } else {
            let now = Instant::now();
            self.value = Some(State {
                current: v,
                peak: v,
                bottom: v,
                last_peak_at: now,
                last_bottom_at: now,
            });
        }
    }

    pub fn get(&self) -> Option<u64> {
        self.value.as_ref().map(|v| v.current)
    }

    /// If set to `Some(Duration)` a peak and bottom values will
    /// be diplayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// If set to None the peak and bottom values will not be shown.
    pub fn set_memorize_extrema(&mut self, d: Duration) {
        self.memorize_extrema = Some(d)
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
        if let Some(ref state) = self.value {
            into.items.push((self.name.clone(), state.current.into()));
            if let Some(ext_dur) = self.memorize_extrema {
                let peak_name = format!("{}_peak", self.name);
                if state.last_peak_at > Instant::now() - ext_dur {
                    into.items.push((peak_name, state.peak.into()));
                } else {
                    into.items.push((peak_name, state.current.into()));
                }
                let bottom_name = format!("{}_bottom", self.name);
                if state.last_bottom_at > Instant::now() - ext_dur {
                    into.items.push((bottom_name, state.bottom.into()));
                } else {
                    into.items.push((bottom_name, state.current.into()));
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

struct State {
    current: u64,
    peak: u64,
    bottom: u64,
    last_peak_at: Instant,
    last_bottom_at: Instant,
}
