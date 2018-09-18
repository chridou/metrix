use std::time::{Duration, Instant};

use exponential_decay_histogram::ExponentialDecayHistogram;

use instruments::{Instrument, Update, Updates};
use snapshot::{ItemKind, Snapshot};
use {Descriptive, PutsSnapshot};

use util;

/// For tracking values. E.g. request latencies
pub struct Histogram {
    name: String,
    title: Option<String>,
    description: Option<String>,
    inner_histogram: ExponentialDecayHistogram,
    last_update: Instant,
    max_inactivity_duration: Option<Duration>,
    reset_after_inactivity: bool,
}

impl Histogram {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Histogram {
        let inner_histogram = ExponentialDecayHistogram::new();
        Histogram {
            name: name.into(),
            title: None,
            description: None,
            inner_histogram,
            last_update: Instant::now(),
            max_inactivity_duration: None,
            reset_after_inactivity: true,
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

    /// Sets the maximum amount of time this histogram may be
    /// inactive until no more snapshots are taken
    ///
    /// Default is no inactivity tracking.
    pub fn set_inactivity_limit(&mut self, limit: Duration) {
        self.max_inactivity_duration = Some(limit);
    }

    /// Reset the histogram if inactivity tracking was enabled
    /// and the histogram was inactive.
    ///
    /// The default is `true`. Only has an effect if a `max_inactivity_duration`
    /// is set.
    pub fn reset_after_inactivity(&mut self, reset: bool) {
        self.reset_after_inactivity = reset;
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot) {
        if let Some(d) = self.max_inactivity_duration {
            if self.last_update.elapsed() > d {
                into.items
                    .push(("_inactive".to_string(), ItemKind::Boolean(true)));
                into.items
                    .push(("_active".to_string(), ItemKind::Boolean(false)));
                return;
            } else {
                into.items
                    .push(("_inactive".to_string(), ItemKind::Boolean(false)));
                into.items
                    .push(("_active".to_string(), ItemKind::Boolean(true)));
            }
        };

        let snapshot = self.inner_histogram.snapshot();

        let histo_snapshot = if snapshot.count() > 0 {
            let quantiles = vec![
                (50u16, snapshot.value(0.5)),
                (75u16, snapshot.value(0.75)),
                (95u16, snapshot.value(0.95)),
                (98u16, snapshot.value(0.98)),
                (99u16, snapshot.value(0.99)),
                (999u16, snapshot.value(0.999)),
            ];

            HistogramSnapshot {
                min: Some(snapshot.min()),
                max: Some(snapshot.max()),
                mean: Some(snapshot.mean()),
                stddev: Some(snapshot.stddev()),
                count: snapshot.count(),
                quantiles: quantiles,
            }
        } else {
            HistogramSnapshot::default()
        };

        histo_snapshot.put_snapshot(into);
    }
}

impl Instrument for Histogram {}

impl PutsSnapshot for Histogram {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
    }
}

impl Updates for Histogram {
    fn update(&mut self, with: &Update) -> usize {
        if let Some(d) = self.max_inactivity_duration {
            if self.reset_after_inactivity && self.last_update.elapsed() > d {
                self.inner_histogram = ExponentialDecayHistogram::new()
            }
        };

        self.last_update = Instant::now();

        match *with {
            Update::ObservationWithValue(v, t) => {
                if t > self.last_update {
                    self.inner_histogram.update_at(t, v as i64);
                    self.last_update = t
                } else {
                    self.inner_histogram.update(v as i64);
                    self.last_update = Instant::now();
                }
                1
            }
            _ => 0,
        }
    }
}

impl Descriptive for Histogram {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

struct HistogramSnapshot {
    pub max: Option<i64>,
    pub min: Option<i64>,
    pub mean: Option<f64>,
    pub stddev: Option<f64>,
    pub count: u64,
    pub quantiles: Vec<(u16, i64)>,
}

impl Default for HistogramSnapshot {
    fn default() -> HistogramSnapshot {
        HistogramSnapshot {
            max: None,
            min: None,
            mean: None,
            stddev: None,
            count: 0,
            quantiles: Vec::new(),
        }
    }
}

impl HistogramSnapshot {
    pub fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("count".to_string(), self.count.into()));

        if let Some(x) = self.max {
            into.items.push(("max".to_string(), x.into()));
        }
        if let Some(x) = self.min {
            into.items.push(("min".to_string(), x.into()));
        }
        if let Some(x) = self.mean {
            into.items.push(("mean".to_string(), x.into()));
        }
        if let Some(x) = self.stddev {
            into.items.push(("stddev".to_string(), x.into()));
        }

        if !self.quantiles.is_empty() {
            let mut quantiles = Snapshot::default();

            for &(ref q, ref v) in &self.quantiles {
                quantiles.items.push((format!("p{}", q), ItemKind::Int(*v)));
            }

            into.items
                .push(("quantiles".to_string(), ItemKind::Snapshot(quantiles)));
        }
    }
}
