use std::cell::Cell;
use std::time::{Duration, Instant};

use metrics::metrics::{Meter as MMeter, StdMeter};

use instruments::{Instrument, Update, Updates};

use snapshot::{ItemKind, Snapshot};
use util;
use {Descriptive, PutsSnapshot};

/// For measuring rates, e.g. request/s
///
/// This meter count occurences. An occurrence with values is
/// counted as 1 occurence.
///
/// To get rates on values use `instruments::other_instruments::ValeMeter`
pub struct Meter {
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

impl Meter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
        Meter {
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

    /// Rates nbelow this value will be shown as zero.
    ///
    /// Default is 0.001
    pub fn set_lower_cuttoff(&mut self, cutoff: f64) {
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
    /// Default: enabled
    pub fn set_five_minute_rate_enabled(&mut self, enabled: bool) {
        self.five_minute_rate_enabled = enabled;
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: enabled
    pub fn set_fifteen_minute_rate_enabled(&mut self, enabled: bool) {
        self.fifteen_minute_rate_enabled = enabled;
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot) {
        if self.last_tick.get().elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick();
            self.last_tick.set(Instant::now());
        }

        let snapshot = self.inner_meter.snapshot();

        let meter_snapshot = MeterSnapshot {
            count: snapshot.count as u64,
            one_minute: if self.one_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[0] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[0]
                    },
                })
            } else {
                None
            },
            five_minutes: if self.one_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[1] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[1]
                    },
                })
            } else {
                None
            },
            fifteen_minutes: if self.one_minute_rate_enabled {
                Some(MeterRate {
                    rate: if snapshot.rates[2] < self.lower_cutoff {
                        0.0
                    } else {
                        snapshot.rates[2]
                    },
                })
            } else {
                None
            },
        };
        meter_snapshot.put_snapshot(into);
    }
}

impl Instrument for Meter {}

impl PutsSnapshot for Meter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
    }
}

impl Updates for Meter {
    fn update(&mut self, with: &Update) {
        if self.last_tick.get().elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick();
            self.last_tick.set(Instant::now());
        }

        match *with {
            Update::ObservationWithValue(_, _) => self.inner_meter.mark(1),
            Update::Observations(n, _) => {
                if n <= ::std::i64::MAX as u64 && n != 0 {
                    self.inner_meter.mark(n as i64)
                }
            }
            Update::Observation(_) => self.inner_meter.mark(1),
        }
    }
}

impl Descriptive for Meter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

pub(crate) struct MeterSnapshot {
    pub count: u64,
    pub one_minute: Option<MeterRate>,
    pub five_minutes: Option<MeterRate>,
    pub fifteen_minutes: Option<MeterRate>,
}

impl MeterSnapshot {
    pub fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("count".to_string(), self.count.into()));

        if let Some(ref one_minute_data) = self.one_minute {
            let mut one_minute = Snapshot::default();
            one_minute_data.put_snapshot(&mut one_minute);
            into.items
                .push(("one_minute".to_string(), ItemKind::Snapshot(one_minute)));
        }

        if let Some(ref five_minute_data) = self.five_minutes {
            let mut five_minutes = Snapshot::default();
            five_minute_data.put_snapshot(&mut five_minutes);
            into.items
                .push(("five_minutes".to_string(), ItemKind::Snapshot(five_minutes)));
        }

        if let Some(ref fifteen_minute_data) = self.fifteen_minutes {
            let mut fifteen_minutes = Snapshot::default();
            fifteen_minute_data.put_snapshot(&mut fifteen_minutes);
            into.items.push((
                "fifteen_minutes".to_string(),
                ItemKind::Snapshot(fifteen_minutes),
            ));
        }
    }
}

pub(crate) struct MeterRate {
    pub rate: f64,
}

impl MeterRate {
    fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("rate".to_string(), self.rate.into()));
    }
}
