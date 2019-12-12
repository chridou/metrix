use std::cell::Cell;
use std::time::{Duration, Instant};

use metrics::metrics::{Meter as MMeter, StdMeter};

use crate::instruments::meter::{MeterRate, MeterSnapshot};
use crate::instruments::{Instrument, InstrumentAdapter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::{Descriptive, ObservedValue, PutsSnapshot, TimeUnit};

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
    display_time_unit: TimeUnit,
}

impl ValueMeter {
    pub fn new<T: Into<String>>(name: T) -> ValueMeter {
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
            display_time_unit: TimeUnit::default(),
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> ValueMeter {
        Self::new(name)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn name<T: Into<String>>(mut self, name: T) -> Self {
        self.set_name(name);
        self
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn title<T: Into<String>>(mut self, title: T) -> Self {
        self.set_title(title);
        self
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn description<T: Into<String>>(mut self, description: T) -> Self {
        self.set_description(description);
        self
    }

    /// Rates below this value will be shown as zero.
    ///
    /// Default is 0.001
    pub fn set_lower_cutoff(&mut self, cutoff: f64) {
        self.lower_cutoff = cutoff
    }

    /// Rates below this value will be shown as zero.
    ///
    /// Default is 0.001
    pub fn lower_cutoff(mut self, cutoff: f64) -> Self {
        self.set_lower_cutoff(cutoff);
        self
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: enabled
    pub fn set_one_minute_rate_enabled(&mut self, enabled: bool) {
        self.one_minute_rate_enabled = enabled;
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: enabled
    pub fn one_minute_rate_enabled(mut self, enabled: bool) -> Self {
        self.set_one_minute_rate_enabled(enabled);
        self
    }

    /// Enable tracking of five minute rates.
    ///
    /// Default: disabled
    pub fn set_five_minute_rate_enabled(&mut self, enabled: bool) {
        self.five_minute_rate_enabled = enabled;
    }

    /// Enable tracking of five minute rates.
    ///
    /// Default: disabled
    pub fn five_minute_rate_enabled(mut self, enabled: bool) -> Self {
        self.set_five_minute_rate_enabled(enabled);
        self
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: disabled
    pub fn set_fifteen_minute_rate_enabled(&mut self, enabled: bool) {
        self.fifteen_minute_rate_enabled = enabled;
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: disabled
    pub fn fifteen_minute_rate_enabled(mut self, enabled: bool) -> Self {
        self.set_fifteen_minute_rate_enabled(enabled);
        self
    }

    pub fn set_display_time_unit(&mut self, display_time_unit: TimeUnit) {
        self.display_time_unit = display_time_unit
    }
    pub fn display_time_unit(mut self, display_time_unit: TimeUnit) -> Self {
        self.set_display_time_unit(display_time_unit);
        self
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations on the given label.
    pub fn for_label<L: Eq>(self, label: L) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::for_label(label, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations with the given labels.
    ///
    /// If `labels` is empty the instrument will not react to any observations
    pub fn for_labels<L: Eq>(self, labels: Vec<L>) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::for_labels(labels, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// all observations.
    pub fn for_all_labels<L: Eq>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::new(self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<L, P>(self, label_predicate: P) -> InstrumentAdapter<L, Self>
    where
        L: Eq,
        P: Fn(&L) -> bool + Send + 'static,
    {
        InstrumentAdapter::by_predicate(label_predicate, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument to no
    /// observations.
    pub fn adapter<L: Eq>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::deaf(self)
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
            Update::ObservationWithValue(observed_value, _) => match observed_value {
                ObservedValue::Duration(time, unit) => {
                    let v =
                        super::super::duration_to_display_value(time, unit, self.display_time_unit);
                    self.inner_meter.mark(v as i64);

                    1
                }
                other => {
                    if let Some(v) = other.convert_to_i64() {
                        self.inner_meter.mark(v);
                        1
                    } else {
                        0
                    }
                }
            },
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
