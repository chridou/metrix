use std::time::{Duration, Instant};
use std::fmt::Display;

use exponential_decay_histogram::ExponentialDecayHistogram;
use metrics::metrics::{Meter as MMeter, StdMeter};

use Observation;
use snapshot::*;

#[derive(Debug, Clone, Copy)]
pub enum ValueScaling {
    NanosToMillis,
    NanosToMicros,
}

/// An update instruction for an instrument
pub enum Update {
    /// Many observations ithout a value at a given time
    Observations(u64, Instant),
    /// One observation without a value at a given time
    Observation(Instant),
    /// One observation with a value at a given time
    ObservationWithValue(u64, Instant),
}

impl Update {
    pub fn scale(self, scaling: ValueScaling) -> Update {
        if let Update::ObservationWithValue(v, t) = self {
            match scaling {
                ValueScaling::NanosToMillis => Update::ObservationWithValue(v / 1_000_000, t),
                ValueScaling::NanosToMicros => Update::ObservationWithValue(v / 1_000, t),
            }
        } else {
            self
        }
    }
}

impl<T> From<Observation<T>> for Update {
    fn from(obs: Observation<T>) -> Update {
        match obs {
            Observation::Observed {
                count, timestamp, ..
            } => Update::Observations(count, timestamp),
            Observation::ObservedOne { timestamp, .. } => Update::Observation(timestamp),
            Observation::ObservedOneValue {
                value, timestamp, ..
            } => Update::ObservationWithValue(value, timestamp),
        }
    }
}

impl<'a, T> From<&'a Observation<T>> for Update {
    fn from(obs: &'a Observation<T>) -> Update {
        match *obs {
            Observation::Observed {
                count, timestamp, ..
            } => Update::Observations(count, timestamp),
            Observation::ObservedOne { timestamp, .. } => Update::Observation(timestamp),
            Observation::ObservedOneValue {
                value, timestamp, ..
            } => Update::ObservationWithValue(value, timestamp),
        }
    }
}

/// Something that can react on `Observation`s where
/// the `Label` is the type of the label.
///
/// You can use this to implement your own Metrics.
pub trait HandlesObservations: Send + 'static {
    type Label: Send + 'static;
    fn handle_observation(&mut self, observation: &Observation<Self::Label>);
    fn name(&self) -> Option<&str>;
    fn snapshot(&self) -> MetricsSnapshot;
}

/// A cockpit groups panels.
///
/// Use a cockpit to group panels that are somehow related.
/// Since the Cockpit is generic over its label you can
/// use an enum as a label for grouping panels easily.
pub struct Cockpit<L> {
    name: Option<String>,
    panels: Vec<(L, Panel)>,
    value_scaling: Option<ValueScaling>,
}

impl<L> Cockpit<L>
where
    L: Display + Clone + Eq + Send + 'static,
{
    pub fn new<T: Into<String>>(
        name: Option<T>,
        value_scaling: Option<ValueScaling>,
    ) -> Cockpit<L> {
        Cockpit {
            name: name.map(Into::into),
            panels: Vec::new(),
            value_scaling,
        }
    }

    pub fn new_with_name<T: Into<String>>(
        name: T,
        value_scaling: Option<ValueScaling>,
    ) -> Cockpit<L> {
        Cockpit::new(Some(name.into()), value_scaling)
    }

    pub fn with_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    pub fn with_value_scaling(&mut self, value_scaling: ValueScaling) {
        self.value_scaling = Some(value_scaling)
    }

    pub fn new_without_name(value_scaling: Option<ValueScaling>) -> Cockpit<L> {
        Cockpit::new::<String>(None, value_scaling)
    }

    pub fn add_panel(&mut self, label: L, panel: Panel) -> bool {
        if self.panels.iter().find(|x| x.0 == label).is_some() {
            false
        } else {
            self.panels.push((label, panel));
            true
        }
    }
}

impl<L> Default for Cockpit<L>
where
    L: Display + Clone + Eq + Send + 'static,
{
    fn default() -> Cockpit<L> {
        Cockpit::new::<String>(None, None)
    }
}

impl<L> HandlesObservations for Cockpit<L>
where
    L: Clone + Display + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) {
        let update: Update = observation.into();

        let update = if let Some(scaling) = self.value_scaling {
            update.scale(scaling)
        } else {
            update
        };

        if let Some(&mut (_, ref mut panel)) =
            self.panels.iter_mut().find(|p| &p.0 == observation.label())
        {
            panel.update(&update)
        }
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let panels: Vec<_> = self.panels
            .iter()
            .map(|&(ref l, ref p)| (l.to_string(), p.snapshot()))
            .collect();
        if let Some(ref name) = self.name {
            MetricsSnapshot::NamedGroup(name.clone(), vec![MetricsSnapshot::Panels(panels)])
        } else {
            MetricsSnapshot::Panels(panels)
        }
    }

    fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }
}

pub trait Updates {
    fn update(&mut self, with: &Update);
}

/// The panel shows recorded
/// observations of the same label
/// in different flavours.
///
/// Let's say you want to monitor the successful requests
/// of a specific endpoint of your REST API.
/// You would then create a panel for this and might
/// want to add a counter and a meter and a histogram
/// to track latencies.
pub struct Panel {
    pub counter: Option<Counter>,
    pub gauge: Option<Gauge>,
    pub meter: Option<Meter>,
    pub histogram: Option<Histogram>,
}

impl Panel {
    pub fn add_counter(&mut self, counter: Counter) {
        self.counter = Some(counter);
    }

    pub fn add_gauge(&mut self, gauge: Gauge) {
        self.gauge = Some(gauge);
    }

    pub fn add_meter(&mut self, meter: Meter) {
        self.meter = Some(meter);
    }

    pub fn add_histogram(&mut self, histogram: Histogram) {
        self.histogram = Some(histogram);
    }

    pub fn snapshot(&self) -> PanelSnapshot {
        PanelSnapshot {
            counter: self.counter.as_ref().map(|x| x.snapshot()),
            gauge: self.gauge.as_ref().map(|x| x.snapshot()),
            meter: self.meter.as_ref().map(|x| x.snapshot()),
            histogram: self.histogram.as_ref().map(|x| x.snapshot()),
        }
    }
}

impl Default for Panel {
    fn default() -> Panel {
        Panel {
            counter: None,
            gauge: None,
            meter: None,
            histogram: None,
        }
    }
}

impl Updates for Panel {
    fn update(&mut self, with: &Update) {
        self.counter.iter_mut().for_each(|x| x.update(with));
        self.gauge.iter_mut().for_each(|x| x.update(with));
        self.meter.iter_mut().for_each(|x| x.update(with));
        self.histogram.iter_mut().for_each(|x| x.update(with));
    }
}

/// A simple ever increasing counter
pub struct Counter {
    name: String,
    count: u64,
}

impl Counter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
            count: 0,
        }
    }

    pub fn inc(&mut self) {
        self.count += 1;
    }

    pub fn inc_by(&mut self, n: u64) {
        self.count += n;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn snapshot(&self) -> (String, CounterSnapshot) {
        (self.name.clone(), CounterSnapshot { count: self.count })
    }
}

impl Updates for Counter {
    fn update(&mut self, with: &Update) {
        match *with {
            Update::Observation(_) => self.inc(),
            Update::Observations(n, _) => self.inc_by(n),
            Update::ObservationWithValue(_, _) => self.inc(),
        }
    }
}

/// Simply returns the value that has been observed last.
pub struct Gauge {
    name: String,
    value: u64,
}

impl Gauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            value: 0,
        }
    }

    pub fn set(&mut self, v: u64) {
        self.value = v;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn snapshot(&self) -> (String, GaugeSnapshot) {
        (self.name.clone(), GaugeSnapshot { value: self.value })
    }
}

impl Updates for Gauge {
    fn update(&mut self, with: &Update) {
        match *with {
            Update::ObservationWithValue(v, _) => self.set(v),
            _ => (),
        }
    }
}

/// For measuring rates, e.g. request/s
pub struct Meter {
    name: String,
    last_tick: Instant,
    inner_meter: StdMeter,
}

impl Meter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
        Meter {
            name: name.into(),
            last_tick: Instant::now(),
            inner_meter: StdMeter::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn snapshot(&self) -> (String, MeterSnapshot) {
        if self.last_tick.elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick()
        }

        let snapshot = self.inner_meter.snapshot();

        (
            self.name.clone(),
            MeterSnapshot {
                one_minute: MeterRate {
                    count: snapshot.count as u64,
                    rate: snapshot.rates[0],
                },
                five_minutes: MeterRate {
                    count: snapshot.count as u64,
                    rate: snapshot.rates[1],
                },
                fifteen_minutes: MeterRate {
                    count: snapshot.count as u64,
                    rate: snapshot.rates[2],
                },
            },
        )
    }
}

impl Updates for Meter {
    fn update(&mut self, with: &Update) {
        if self.last_tick.elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick()
        }

        match *with {
            Update::ObservationWithValue(_, _) => self.inner_meter.mark(1),
            Update::Observations(n, _) => self.inner_meter.mark(n as i64),
            _ => (),
        }
    }
}

/// For tracking values. E.g. request latencies
pub struct Histogram {
    inner_histogram: ExponentialDecayHistogram,
    name: String,
    last_update: Instant,
}

impl Histogram {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Histogram {
        let inner_histogram = ExponentialDecayHistogram::new();
        Histogram {
            name: name.into(),
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

    pub fn snapshot(&self) -> (String, HistogramSnapshot) {
        let snapshot = self.inner_histogram.snapshot();

        let quantiles = vec![
            (50u16, snapshot.value(0.5)),
            (75u16, snapshot.value(0.75)),
            (99u16, snapshot.value(0.99)),
            (999u16, snapshot.value(0.999)),
        ];
        (
            self.name.clone(),
            HistogramSnapshot {
                min: snapshot.min(),
                max: snapshot.max(),
                mean: snapshot.mean(),
                stddev: snapshot.stddev(),
                count: snapshot.count(),
                quantiles: quantiles,
            },
        )
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
