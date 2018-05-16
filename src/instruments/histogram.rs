use std::time::Instant;

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

    fn put_values_into_snapshot(&self, into: &mut Snapshot) {
        let snapshot = self.inner_histogram.snapshot();

        let quantiles = vec![
            (50u16, snapshot.value(0.5)),
            (75u16, snapshot.value(0.75)),
            (99u16, snapshot.value(0.99)),
            (999u16, snapshot.value(0.999)),
        ];

        let histo_snapshot = HistogramSnapshot {
            min: snapshot.min(),
            max: snapshot.max(),
            mean: snapshot.mean(),
            stddev: snapshot.stddev(),
            count: snapshot.count(),
            quantiles: quantiles,
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
    fn update(&mut self, with: &Update) {
        match *with {
            Update::ObservationWithValue(v, t) => if t > self.last_update {
                self.inner_histogram.update_at(t, v as i64);
                self.last_update = t
            } else {
                self.inner_histogram.update(v as i64);
                self.last_update = Instant::now();
            },
            _ => (),
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
    pub max: i64,
    pub min: i64,
    pub mean: f64,
    pub stddev: f64,
    pub count: u64,
    pub quantiles: Vec<(u16, i64)>,
}

impl HistogramSnapshot {
    pub fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("max".to_string(), self.max.into()));
        into.items.push(("min".to_string(), self.min.into()));
        into.items.push(("mean".to_string(), self.mean.into()));
        into.items.push(("stddev".to_string(), self.stddev.into()));
        into.items.push(("count".to_string(), self.count.into()));

        let mut quantiles = Snapshot::default();

        for &(ref q, ref v) in &self.quantiles {
            quantiles.items.push((format!("p{}", q), ItemKind::Int(*v)));
        }

        into.items
            .push(("quantiles".to_string(), ItemKind::Snapshot(quantiles)));
    }
}
