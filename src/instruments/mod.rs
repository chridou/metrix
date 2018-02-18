//! Instruments that track values and/or derive values
//! from observations.
use std::time::Instant;

use Observation;
use snapshot::{ItemKind, Snapshot};
use {Descriptive, PutsSnapshot};
use util;

pub use self::counter::*;
pub use self::gauge::*;
pub use self::meter::*;
pub use self::histogram::*;

mod counter;
mod gauge;
mod meter;
mod histogram;

/// Scales incoming values.
///
/// This can be used either on a `Cockpit`
/// or on a `Panel`. Be carefule when using on both
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
    /// Many observations ithout a value at a given time
    Observations(u64, Instant),
    /// One observation without a value at a given time
    Observation(Instant),
    /// One observation with a value at a given time
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
/// This is basically an explodes `Observation`
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

impl<'a, T> From<&'a Observation<T>> for LabelAndUpdate<T>
where
    T: Clone,
{
    fn from(obs: &'a Observation<T>) -> LabelAndUpdate<T> {
        match *obs {
            Observation::Observed {
                ref label,
                count,
                timestamp,
                ..
            } => LabelAndUpdate(label.clone(), Update::Observations(count, timestamp)),
            Observation::ObservedOne {
                ref label,
                timestamp,
                ..
            } => LabelAndUpdate(label.clone(), Update::Observation(timestamp)),
            Observation::ObservedOneValue {
                ref label,
                value,
                timestamp,
                ..
            } => LabelAndUpdate(
                label.clone(),
                Update::ObservationWithValue(value, timestamp),
            ),
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

/// The panel shows recorded
/// observations of the same label
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
/// struct DemoLabel;
///
/// let counter = Counter::new_with_defaults("the_counter");
/// let gauge = Gauge::new_with_defaults("the_gauge");
///
/// assert_eq!(0, counter.get());
/// assert_eq!(None, gauge.get());
///
/// let mut panel = Panel::new(DemoLabel);
/// panel.set_counter(counter);
/// panel.set_gauge(gauge);
///
/// let update = Update::ObservationWithValue(12, Instant::now());
/// panel.update(&update);
///
/// assert_eq!(Some(1), panel.counter().map(|c| c.get()));
/// assert_eq!(Some(12), panel.gauge().and_then(|g| g.get()));
/// ```
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

    pub fn set_gauge(&mut self, gauge: Gauge) {
        self.gauge = Some(gauge);
    }

    pub fn set_meter(&mut self, meter: Meter) {
        self.meter = Some(meter);
    }

    pub fn set_histogram(&mut self, histogram: Histogram) {
        self.histogram = Some(histogram);
    }

    pub fn counter(&self) -> Option<&Counter> {
        self.counter.as_ref()
    }

    pub fn gauge(&self) -> Option<&Gauge> {
        self.gauge.as_ref()
    }

    pub fn meter(&self) -> Option<&Meter> {
        self.meter.as_ref()
    }

    pub fn histogram(&self) -> Option<&Histogram> {
        self.histogram.as_ref()
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
    /// The name is path segment within a `Snapshot`
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
}

impl<L> PutsSnapshot for Panel<L> {
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
