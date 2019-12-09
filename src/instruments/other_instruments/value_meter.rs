use std::cell::Cell;
use std::time::{Duration, Instant};

use metrics::metrics::{Meter as MMeter, StdMeter};

use crate::instruments::meter::{MeterRate, MeterSnapshot};
use crate::instruments::{Instrument, Update, Updates};
use crate::snapshot::Snapshot;
use crate::{Descriptive, PutsSnapshot};

/// A meter that is ticked by values instead of observations
pub struct ValueMeter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    last_tick: Cell<Instant>,
    inner_meter: StdMeter,
    lower_cutoff: f64,
    one_minute_rate_enabled: bool,
    five_minute_rate_enabled: bool,
    fifteen_minute_rate_enabled: bool,
}

impl ValueMeter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> ValueMeter {
        ValueMeter {
            name: name.into(),
            title: None,
            description: None,
            last_tick: Cell::new(Instant::now()),
            inner_meter: StdMeter::default(),
            lower_cutoff: 0.001,
            one_minute_rate_enabled: true,
            five_minute_rate_enabled: false,
            fifteen_minute_rate_enabled: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    /// Rates below this value will be shown as zero.
    ///
    /// Default is 0.001
    pub fn set_lower_cutoff(&mut self, cutoff: f64) {
        self.lower_cutoff = cutoff
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: enabled
    pub fn set_one_minute_rate_enabled(&mut self, enabled: bool) {
        self.one_minute_rate_enabled = enabled;
    }

    /// Enable tracking of five minute rates.
    ///
    /// Default: disabled
    pub fn set_five_minute_rate_enabled(&mut self, enabled: bool) {
        self.five_minute_rate_enabled = enabled;
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: disabled
    pub fn set_fifteen_minute_rate_enabled(&mut self, enabled: bool) {
        self.fifteen_minute_rate_enabled = enabled;
    }

    pub(crate) fn get_snapshot(&self) -> MeterSnapshot {
        if self.last_tick.get().elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick();
            self.last_tick.set(Instant::now());
        }

        let snapshot = self.inner_meter.snapshot();

        let meter_snapshot = MeterSnapshot {
            name: &self.name,
            title: self.title.as_ref().map(|x| &**x),
            description: self.description.as_ref().map(|x| &**x),
            count: snapshot.count as u64,
            one_minute: if self.one_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[0] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[0]
                    },
                    share: None,
                })
            } else {
                None
            },
            five_minutes: if self.five_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[1] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[1]
                    },
                    share: None,
                })
            } else {
                None
            },
            fifteen_minutes: if self.fifteen_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[2] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[2]
                    },
                    share: None,
                })
            } else {
                None
            },
        };

        meter_snapshot
    }
}

impl Instrument for ValueMeter {}

impl PutsSnapshot for ValueMeter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        let meter_snapshot = self.get_snapshot();

        meter_snapshot.put_snapshot(into, descriptive);
    }
}

impl Updates for ValueMeter {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(v, _) => {
                if v <= ::std::i64::MAX as u64 && v != 0 {
                    self.inner_meter.mark(v as i64)
                }
                1
            }
            _ => 0,
        }
    }
}

impl Descriptive for ValueMeter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
