use std::cell::Cell;
use std::time::{Duration, Instant};

use crate::instruments::fundamentals::metrics_meter::{Meter as MMeter, StdMeter};

use crate::instruments::{
    AcceptAllLabels, Instrument, InstrumentAdapter, LabelFilter, LabelPredicate, Update, Updates,
};
use crate::snapshot::{ItemKind, Snapshot};
use crate::util;
use crate::{Descriptive, PutsSnapshot};

/// For measuring rates, e.g. request/s
///
/// This meter count occurrences. An occurrence with values is
/// counted as 1 occurrence.
///
/// To get rates on values use `instruments::other_instruments::ValueMeter`
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
    pub fn new<T: Into<String>>(name: T) -> Meter {
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

    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
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

    pub fn accept<L: Eq + Send + 'static, F: Into<LabelFilter<L>>>(
        self,
        accept: F,
    ) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::accept(accept, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations on the given label.
    pub fn for_label<L: Eq + Send + 'static>(self, label: L) -> InstrumentAdapter<L, Self> {
        self.accept(label)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations with the given labels.
    ///
    /// If `labels` is empty the instrument will not react to any observations
    pub fn for_labels<L: Eq + Send + 'static>(self, labels: Vec<L>) -> InstrumentAdapter<L, Self> {
        self.accept(labels)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// all observations.
    pub fn for_all_labels<L: Eq + Send + 'static>(self) -> InstrumentAdapter<L, Self> {
        self.accept(AcceptAllLabels)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<L, P>(self, label_predicate: P) -> InstrumentAdapter<L, Self>
    where
        L: Eq + Send + 'static,
        P: Fn(&L) -> bool + Send + 'static,
    {
        self.accept(LabelPredicate(label_predicate))
    }

    /// Creates an `InstrumentAdapter` that makes this instrument to no
    /// observations.
    pub fn adapter<L: Eq + Send + 'static>(self) -> InstrumentAdapter<L, Self> {
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
            title: self.title.as_deref(),
            description: self.description.as_deref(),
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

impl Instrument for Meter {}

impl PutsSnapshot for Meter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        let meter_snapshot = self.get_snapshot();

        meter_snapshot.put_snapshot(into, descriptive);
    }
}

impl Updates for Meter {
    fn update(&mut self, with: &Update) -> usize {
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

        1
    }
}

impl Descriptive for Meter {
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub(crate) struct MeterSnapshot<'a> {
    pub name: &'a str,
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub count: u64,
    pub one_minute: Option<MeterRate>,
    pub five_minutes: Option<MeterRate>,
    pub fifteen_minutes: Option<MeterRate>,
}

impl<'a> MeterSnapshot<'a> {
    pub fn put_snapshot(&self, into_container: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into_container, descriptive);
        let mut new_level = Snapshot::default();

        new_level
            .items
            .push(("count".to_string(), self.count.into()));

        if let Some(ref one_minute_data) = self.one_minute {
            let mut one_minute = Snapshot::default();
            one_minute_data.put_snapshot(&mut one_minute);
            new_level
                .items
                .push(("one_minute".to_string(), ItemKind::Snapshot(one_minute)));
        }

        if let Some(ref five_minute_data) = self.five_minutes {
            let mut five_minutes = Snapshot::default();
            five_minute_data.put_snapshot(&mut five_minutes);
            new_level
                .items
                .push(("five_minutes".to_string(), ItemKind::Snapshot(five_minutes)));
        }

        if let Some(ref fifteen_minute_data) = self.fifteen_minutes {
            let mut fifteen_minutes = Snapshot::default();
            fifteen_minute_data.put_snapshot(&mut fifteen_minutes);
            new_level.items.push((
                "fifteen_minutes".to_string(),
                ItemKind::Snapshot(fifteen_minutes),
            ));
        }

        into_container.push(self.name, ItemKind::Snapshot(new_level));
    }
}

impl<'a> Descriptive for MeterSnapshot<'a> {
    fn title(&self) -> Option<&str> {
        self.title
    }

    fn description(&self) -> Option<&str> {
        self.description
    }
}

pub(crate) struct MeterRate {
    pub rate: f64,
    pub share: Option<f64>,
}

impl MeterRate {
    fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("rate".to_string(), self.rate.into()));
        if let Some(share) = self.share {
            into.items.push(("share".to_string(), share.into()));
        }
    }
}
