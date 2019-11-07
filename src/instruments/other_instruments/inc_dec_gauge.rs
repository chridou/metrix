use std::time::{Duration, Instant};

use instruments::{Instrument, Update, Updates};
use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot, DECR, INCR};

pub struct IncDecGauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: State,
    memorize_extrema: Option<Duration>,
}

impl IncDecGauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> IncDecGauge {
        Self {
            name: name.into(),
            title: None,
            description: None,
            value: State {
                current: 0,
                peak: 0,
                bottom: 0,
                last_peak_at: Instant::now(),
                last_bottom_at: Instant::now(),
            },
            memorize_extrema: None,
        }
    }

    fn modify(&mut self, incr: bool) {
        let state = &mut self.value;

        let v = if incr {
            state.current + 1
        } else {
            state.current - 1
        };

        if let Some(ext_dur) = self.memorize_extrema {
            let now = Instant::now();
            if v >= state.peak {
                state.last_peak_at = now;
                state.peak = v;
            } else if state.last_peak_at < now - ext_dur {
                state.peak = v;
            }

            if v <= state.bottom {
                state.last_bottom_at = now;
                state.bottom = v;
            } else if state.last_bottom_at < now - ext_dur {
                state.bottom = v;
            }
        }
        state.current = v;
    }

    pub fn get(&self) -> i64 {
        self.value.current
    }

    /// If set to `Some(Duration)` a peak and bottom values will
    /// be displayed for the given duration unless there
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

impl Instrument for IncDecGauge {}

impl PutsSnapshot for IncDecGauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        let state = &self.value;
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

impl Updates for IncDecGauge {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(v, _) => {
                match v {
                    INCR => self.modify(true),
                    DECR => self.modify(false),
                    _ => {}
                }
                1
            }
            _ => 0,
        }
    }
}

impl Descriptive for IncDecGauge {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

struct State {
    current: i64,
    peak: i64,
    bottom: i64,
    last_peak_at: Instant,
    last_bottom_at: Instant,
}
