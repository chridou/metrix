//! Instruments that track values and/or derive values
//! from observations.
use std::time::Instant;

use self::switches::*;
use Observation;
use snapshot::{ItemKind, Snapshot};
use util;
use {Descriptive, PutsSnapshot};

pub use self::counter::Counter;
pub use self::gauge::Gauge;
pub use self::histogram::Histogram;
pub use self::meter::Meter;

mod counter;
mod gauge;
mod histogram;
mod meter;
pub mod other_instruments;
pub mod polled;
pub mod switches;

/// Scales incoming values.
///
/// This can be used either with a `Cockpit`
/// or with a `Panel`. Be careful when using on both
/// since both components do not care whether the
/// other already scaled a value.
#[derive(Debug, Clone, Copy)]
pub enum ValueScaling {
    /// Consider incoming values nanos and make them millis
    NanosToMillis,
    /// Consider incoming values nanos and make them micros
    NanosToMicros,
}

#[derive(Debug, Clone)]
/// An update instruction for an instrument
pub enum Update {
    /// Many observations without a value observed at a given time
    Observations(u64, Instant),
    /// One observation without a value observed at a given time
    Observation(Instant),
    /// One observation with a value observed at a given time
    ObservationWithValue(u64, Instant),
}

impl Update {
    /// Scale by the given `ValueScaling`
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

/// A label with the associated `Update`
///
/// This is basically a split `Observation`
pub struct LabelAndUpdate<T>(pub T, pub Update);

impl<T> From<Observation<T>> for LabelAndUpdate<T> {
    fn from(obs: Observation<T>) -> LabelAndUpdate<T> {
        match obs {
            Observation::Observed {
                label,
                count,
                timestamp,
                ..
            } => LabelAndUpdate(label, Update::Observations(count, timestamp)),
            Observation::ObservedOne {
                label, timestamp, ..
            } => LabelAndUpdate(label, Update::Observation(timestamp)),
            Observation::ObservedOneValue {
                label,
                value,
                timestamp,
                ..
            } => LabelAndUpdate(label, Update::ObservationWithValue(value, timestamp)),
        }
    }
}

/// Implementors of `Updates`
/// can handle `Update`s.
///
/// `Update`s are basically observations without a label.
pub trait Updates {
    /// Update the internal state according to the given `Update`.
    ///
    /// Not all `Update`s might modify the internal state.
    /// Only those that are appropriate and meaningful for
    /// the implementor.
    fn update(&mut self, with: &Update);
}

/// Requirement for an instrument
pub trait Instrument: Updates + PutsSnapshot {}

/// The panel shows recorded
/// observations with the same label
/// in different representations.
///
/// Let's say you want to monitor the successful requests
/// of a specific endpoint of your REST API.
/// You would then create a panel for this and might
/// want to add a counter and a meter and a histogram
/// to track latencies.
///
/// # Example
///
/// ```
/// use std::time::Instant;
/// use metrix::instruments::*;
///
/// #[derive(Clone, PartialEq, Eq)]
/// struct SuccessfulRequests;
///
/// let counter = Counter::new_with_defaults("count");
/// let gauge = Gauge::new_with_defaults("last_latency");
/// let meter = Meter::new_with_defaults("per_second");
/// let histogram = Histogram::new_with_defaults("latencies");
///
/// assert_eq!(0, counter.get());
/// assert_eq!(None, gauge.get());
///
/// let mut panel = Panel::with_name(SuccessfulRequests, "succesful_requests");
/// panel.set_counter(counter);
/// panel.set_gauge(gauge);
/// panel.set_meter(meter);
/// panel.set_histogram(histogram);
///
/// let update = Update::ObservationWithValue(12, Instant::now());
/// panel.update(&update);
///
/// assert_eq!(Some(1), panel.counter().map(|c| c.get()));
/// assert_eq!(Some(12), panel.gauge().and_then(|g| g.get()));
/// ```
pub struct Panel<L> {
    label: L,
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    counter: Option<Counter>,
    gauge: Option<Gauge>,
    meter: Option<Meter>,
    histogram: Option<Histogram>,
    instruments: Vec<Box<Instrument>>,
    snapshooters: Vec<Box<PutsSnapshot>>,
    value_scaling: Option<ValueScaling>,
}

impl<L> Panel<L> {
    /// Create a new `Panel` without a name.
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
            instruments: Vec::new(),
            snapshooters: Vec::new(),
        }
    }

    /// Create a new `Panel` with the given name
    pub fn with_name<T: Into<String>>(label: L, name: T) -> Panel<L> {
        let mut panel = Panel::new(label);
        panel.set_name(name);
        panel
    }

    pub fn set_counter(&mut self, counter: Counter) {
        self.counter = Some(counter);
    }

    pub fn counter(&self) -> Option<&Counter> {
        self.counter.as_ref()
    }

    pub fn set_gauge(&mut self, gauge: Gauge) {
        self.gauge = Some(gauge);
    }

    pub fn gauge(&self) -> Option<&Gauge> {
        self.gauge.as_ref()
    }

    pub fn set_meter(&mut self, meter: Meter) {
        self.meter = Some(meter);
    }

    pub fn meter(&self) -> Option<&Meter> {
        self.meter.as_ref()
    }

    pub fn set_histogram(&mut self, histogram: Histogram) {
        self.histogram = Some(histogram);
    }

    pub fn histogram(&self) -> Option<&Histogram> {
        self.histogram.as_ref()
    }

    #[deprecated(since = "0.6.0", note = "use add_instrument")]
    pub fn set_staircase_timer(&mut self, timer: StaircaseTimer) {
        self.add_instrument(timer);
    }

    #[deprecated(since = "0.6.0", note = "there will be no replacement")]
    pub fn staircase_timer(&self) -> Option<&StaircaseTimer> {
        None
    }

    pub fn add_snapshooter<T: PutsSnapshot>(&mut self, snapshooter: T) {
        self.snapshooters.push(Box::new(snapshooter));
    }

    pub fn snapshooters(&self) -> Vec<&PutsSnapshot> {
        self.snapshooters.iter().map(|p| &**p).collect()
    }

    pub fn add_instrument<I: Instrument>(&mut self, instrument: I) {
        self.instruments.push(Box::new(instrument));
    }

    pub fn instruments(&self) -> Vec<&Instrument> {
        self.instruments.iter().map(|p| &**p).collect()
    }

    pub fn set_value_scaling(&mut self, value_scaling: ValueScaling) {
        self.value_scaling = Some(value_scaling)
    }

    /// Gets the name of this `Panel`
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    /// Set the name if this `Panel`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into());
    }

    /// Sets the `title` of this `Panel`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `Panel`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn label(&self) -> &L {
        &self.label
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
        self.snapshooters
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));
        self.instruments
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));
    }
}

impl<L> PutsSnapshot for Panel<L>
where
    L: Send + 'static,
{
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
        self.instruments.iter_mut().for_each(|x| x.update(&with));
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
