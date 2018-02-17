use std::time::{Duration, Instant};

use exponential_decay_histogram::ExponentialDecayHistogram;
use metrics::metrics::{Meter as MMeter, StdMeter};

use Observation;
use snapshot::*;
use util;

/// Scales incoming values.
///
/// This can be used either on a `Cockpit`
/// or on a `Panel`. Be carefule when using on both
/// since both components do not care whether the
/// other already scaled a value.
#[derive(Debug, Clone, Copy)]
pub enum ValueScaling {
    NanosToMillis,
    NanosToMicros,
}

#[derive(Debug, Clone)]
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

pub struct LabelAndObservation<T>(pub T, pub Update);

impl<T> From<Observation<T>> for LabelAndObservation<T> {
    fn from(obs: Observation<T>) -> LabelAndObservation<T> {
        match obs {
            Observation::Observed {
                label,
                count,
                timestamp,
                ..
            } => LabelAndObservation(label, Update::Observations(count, timestamp)),
            Observation::ObservedOne {
                label, timestamp, ..
            } => LabelAndObservation(label, Update::Observation(timestamp)),
            Observation::ObservedOneValue {
                label,
                value,
                timestamp,
                ..
            } => LabelAndObservation(label, Update::ObservationWithValue(value, timestamp)),
        }
    }
}

impl<'a, T> From<&'a Observation<T>> for LabelAndObservation<T>
where
    T: Clone,
{
    fn from(obs: &'a Observation<T>) -> LabelAndObservation<T> {
        match *obs {
            Observation::Observed {
                ref label,
                count,
                timestamp,
                ..
            } => LabelAndObservation(label.clone(), Update::Observations(count, timestamp)),
            Observation::ObservedOne {
                ref label,
                timestamp,
                ..
            } => LabelAndObservation(label.clone(), Update::Observation(timestamp)),
            Observation::ObservedOneValue {
                ref label,
                value,
                timestamp,
                ..
            } => LabelAndObservation(
                label.clone(),
                Update::ObservationWithValue(value, timestamp),
            ),
        }
    }
}

/// Something that has a title and a description
pub trait Descriptive {
    fn title(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        None
    }
}

/// Something that can react on `Observation`s where
/// the `Label` is the type of the label.
///
/// You can use this to implement your own Metrics.
pub trait HandlesObservations: Send + 'static {
    type Label: Send + 'static;
    fn handle_observation(&mut self, observation: &Observation<Self::Label>);
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool);
}

/// A cockpit groups panels.
///
/// Use a cockpit to group panels that are somehow related.
/// Since the Cockpit is generic over its label you can
/// use an enum as a label for grouping panels easily.
pub struct Cockpit<L> {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    panels: Vec<Panel<L>>,
    value_scaling: Option<ValueScaling>,
}

impl<L> Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    pub fn new<T: Into<String>>(name: T, value_scaling: Option<ValueScaling>) -> Cockpit<L> {
        let mut cockpit = Cockpit::default();
        cockpit.name = Some(name.into());
        cockpit.value_scaling = value_scaling;
        cockpit
    }

    pub fn without_name(value_scaling: Option<ValueScaling>) -> Cockpit<L> {
        let mut cockpit = Cockpit::default();
        cockpit.value_scaling = value_scaling;
        cockpit
    }

    pub fn with_value_scaling<T: Into<String>>(
        &mut self,
        name: T,
        value_scaling: ValueScaling,
    ) -> Cockpit<L> {
        Cockpit::new(name, Some(value_scaling))
    }
    pub fn without_value_scaling<T: Into<String>>(&mut self, name: T) -> Cockpit<L> {
        Cockpit::new(name, None)
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn set_value_scaling(&mut self, value_scaling: ValueScaling) {
        self.value_scaling = Some(value_scaling)
    }

    pub fn add_panel(&mut self, panel: Panel<L>) -> bool {
        if self.panels
            .iter()
            .find(|x| x.label == panel.label)
            .is_some()
        {
            false
        } else {
            self.panels.push(panel);
            true
        }
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        self.panels
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive))
    }
}

impl<L> Default for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn default() -> Cockpit<L> {
        Cockpit {
            name: None,
            title: None,
            description: None,
            panels: Vec::new(),
            value_scaling: None,
        }
    }
}

impl<L> HandlesObservations for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) {
        let LabelAndObservation(label, update) = observation.into();

        let update = if let Some(scaling) = self.value_scaling {
            update.scale(scaling)
        } else {
            update
        };

        self.panels
            .iter_mut()
            .filter(|p| p.label == label)
            .for_each(|p| p.update(&update));
    }

    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if let Some(ref name) = self.name {
            let mut new_level = Snapshot::default();
            self.put_values_into_snapshot(&mut new_level, descriptive);
            into.items
                .push((name.clone(), ItemKind::Snapshot(new_level)));
        } else {
            self.put_values_into_snapshot(into, descriptive);
        }
    }
}

impl<L> Descriptive for Cockpit<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
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
pub struct Panel<L> {
    pub label: L,
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub counter: Option<Counter>,
    pub gauge: Option<Gauge>,
    pub meter: Option<Meter>,
    pub histogram: Option<Histogram>,
    pub value_scaling: Option<ValueScaling>,
}

impl<L> Panel<L> {
    pub fn new(label: L) -> Panel<L> {
        Panel {
            label: label,
            name: None,
            title: None,
            description: None,
            counter: None,
            gauge: None,
            meter: None,
            histogram: None,
            value_scaling: None,
        }
    }

    pub fn with_name<T: Into<String>>(label: L, name: T) -> Panel<L> {
        let mut panel = Panel::new(label);
        panel.set_name(name);
        panel
    }

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

    pub fn set_value_scaling(&mut self, value_scaling: ValueScaling) {
        self.value_scaling = Some(value_scaling)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        self.counter
            .as_ref()
            .iter()
            .for_each(|x| x.put_snapshot(into, descriptive));
        self.gauge
            .as_ref()
            .iter()
            .for_each(|x| x.put_snapshot(into, descriptive));
        self.meter
            .as_ref()
            .iter()
            .for_each(|x| x.put_snapshot(into, descriptive));
        self.histogram
            .as_ref()
            .iter()
            .for_each(|x| x.put_snapshot(into, descriptive));
    }

    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if let Some(ref name) = self.name {
            let mut new_level = Snapshot::default();
            self.put_values_into_snapshot(&mut new_level, descriptive);
            into.items
                .push((name.clone(), ItemKind::Snapshot(new_level)));
        } else {
            self.put_values_into_snapshot(into, descriptive);
        }
    }
}

impl<L> Updates for Panel<L> {
    fn update(&mut self, with: &Update) {
        let with = if let Some(scaling) = self.value_scaling {
            with.clone().scale(scaling)
        } else {
            with.clone()
        };
        self.counter.iter_mut().for_each(|x| x.update(&with));
        self.gauge.iter_mut().for_each(|x| x.update(&with));
        self.meter.iter_mut().for_each(|x| x.update(&with));
        self.histogram.iter_mut().for_each(|x| x.update(&with));
    }
}

impl<L> Descriptive for Panel<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

/// A simple ever increasing counter
pub struct Counter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    count: u64,
}

impl Counter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
            title: None,
            description: None,
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

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_snapshot(&self, into: &mut Snapshot, _descriptive: bool) {
        util::put_prefixed_descriptives(self, &self.name, into);
        into.items
            .push((self.name.clone(), ItemKind::UInt(self.count)));
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

impl Descriptive for Counter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

/// Simply returns the value that has been observed last.
pub struct Gauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<u64>,
}

impl Gauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
        }
    }

    pub fn set(&mut self, v: u64) {
        self.value = Some(v);
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_snapshot(&self, into: &mut Snapshot, _descriptive: bool) {
        if let Some(v) = self.value {
            util::put_prefixed_descriptives(self, &self.name, into);
            into.items.push((self.name.clone(), ItemKind::UInt(v)));
        }
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

impl Descriptive for Gauge {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

/// For measuring rates, e.g. request/s
pub struct Meter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    last_tick: Instant,
    inner_meter: StdMeter,
}

impl Meter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
        Meter {
            name: name.into(),
            title: None,
            description: None,
            last_tick: Instant::now(),
            inner_meter: StdMeter::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_prefixed_descriptives(self, &self.name, into);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level, descriptive);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if self.last_tick.elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick()
        }

        let snapshot = self.inner_meter.snapshot();

        let meter_snapshot = MeterSnapshot {
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
        };
        meter_snapshot.put_snapshot(into, descriptive);
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

impl Descriptive for Meter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

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

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_prefixed_descriptives(self, &self.name, into);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level, descriptive);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
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

        histo_snapshot.put_snapshot(into, descriptive);
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
