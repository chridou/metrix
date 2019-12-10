//! Instruments that track values and/or derive values
//! from observations.
use std::time::{Duration, Instant};

use crate::snapshot::{ItemKind, Snapshot};
use crate::util;
use crate::{Descriptive, HandlesObservations, Observation, ObservedValue, PutsSnapshot, TimeUnit};

pub use self::counter::Counter;
pub use self::gauge::*;
pub use self::histogram::Histogram;
pub use self::meter::Meter;
pub use self::other_instruments::*;
pub use self::polled::*;
pub use self::switches::*;
pub use crate::cockpit::Cockpit;

mod counter;
mod gauge;
mod histogram;
mod meter;
pub mod other_instruments;
pub mod polled;
pub mod switches;

#[derive(Debug, Clone)]
/// An update instruction for an instrument
pub enum Update {
    /// Many observations without a value observed at a given time
    Observations(u64, Instant),
    /// One observation without a value observed at a given time
    Observation(Instant),
    /// One observation with a value observed at a given time
    ObservationWithValue(ObservedValue, Instant),
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

/// A label with the associated `Update`
///
/// This is basically a split `Observation`
pub struct BorrowedLabelAndUpdate<'a, T: 'a>(pub &'a T, pub Update);

impl<'a, T> From<&'a Observation<T>> for BorrowedLabelAndUpdate<'a, T> {
    fn from(obs: &'a Observation<T>) -> BorrowedLabelAndUpdate<'a, T> {
        match obs {
            Observation::Observed {
                label,
                count,
                timestamp,
                ..
            } => BorrowedLabelAndUpdate(label, Update::Observations(*count, *timestamp)),
            Observation::ObservedOne {
                label, timestamp, ..
            } => BorrowedLabelAndUpdate(label, Update::Observation(*timestamp)),
            Observation::ObservedOneValue {
                label,
                value,
                timestamp,
                ..
            } => BorrowedLabelAndUpdate(label, Update::ObservationWithValue(*value, *timestamp)),
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
    ///
    /// Returns the number of instruments updated
    fn update(&mut self, with: &Update) -> usize;
}

/// Requirement for an instrument
pub trait Instrument: Updates + PutsSnapshot {}

//impl Instrument for Box<dyn Instrument + Send + 'static> {}

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
/// use metrix::{HandlesObservations, Observation};
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
/// let mut panel = Panel::named(SuccessfulRequests, "successful_requests");
/// panel.set_counter(counter);
/// panel.set_gauge(gauge);
/// panel.set_meter(meter);
/// panel.set_histogram(histogram);
///
/// let observation = Observation::ObservedOneValue {
///        label: SuccessfulRequests,
///        value: 12.into(),
///        timestamp: Instant::now(),
/// };
/// panel.handle_observation(&observation);
///
/// assert_eq!(Some(1), panel.get_counter().map(|c| c.get()));
/// assert_eq!(Some(12), panel.get_gauge().and_then(|g| g.get()));
/// ```
pub struct Panel<L> {
    label_filter: LabelFilter<L>,
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    counter: Option<InstrumentAdapter<L, Counter>>,
    gauge: Option<GaugeAdapter<L>>,
    meter: Option<InstrumentAdapter<L, Meter>>,
    histogram: Option<InstrumentAdapter<L, Histogram>>,
    panels: Vec<Panel<L>>,
    handlers: Vec<Box<dyn HandlesObservations<Label = L>>>,
    snapshooters: Vec<Box<dyn PutsSnapshot>>,
    last_update: Instant,
    max_inactivity_duration: Option<Duration>,
}

impl<L> Panel<L>
where
    L: Clone + Eq + Send + 'static,
{
    /// Create a new `Panel` without a name which dispatches observations
    /// with the given label
    pub fn new(label: L) -> Panel<L> {
        let mut panel = Panel::accept_all();
        panel.label_filter = LabelFilter::new(label);
        panel
    }

    /// Create a new `Panel` with the given name which dispatches observations
    /// with the given label
    #[deprecated(since = "0.9.24", note = "use 'named'")]
    pub fn with_name<T: Into<String>>(label: L, name: T) -> Panel<L> {
        let mut panel = Panel::accept_all_named(name);
        panel.label_filter = LabelFilter::new(label);
        panel
    }
    /// Create a new `Panel` with the given name which dispatches observations
    /// with the given label
    pub fn named<T: Into<String>>(label: L, name: T) -> Panel<L> {
        let mut panel = Panel::accept_all_named(name);
        panel.label_filter = LabelFilter::new(label);
        panel
    }

    /// Create a new `Panel` without a name which dispatches observations
    /// with the given labels
    pub fn accept(labels: Vec<L>) -> Self {
        let mut panel = Panel::accept_all();
        panel.label_filter = LabelFilter::many(labels);
        panel
    }

    /// Create a new `Panel` with the given name which dispatches observations
    /// with the given labels
    pub fn accept_named<T: Into<String>>(labels: Vec<L>, name: T) -> Self {
        let mut panel = Panel::accept_all_named(name);
        panel.label_filter = LabelFilter::many(labels);
        panel
    }

    /// Create a new `Panel` with the given name which dispatches all
    /// observations
    pub fn accept_all_named<T: Into<String>>(name: T) -> Panel<L> {
        let mut panel = Panel::accept_all();
        panel.name = Some(name.into());
        panel
    }

    /// Create a new `Panel` without a name which dispatches all
    /// observations
    pub fn accept_all() -> Panel<L> {
        Panel {
            label_filter: LabelFilter::AcceptAll,
            name: None,
            title: None,
            description: None,
            counter: None,
            gauge: None,
            meter: None,
            histogram: None,
            panels: Vec::new(),
            handlers: Vec::new(),
            snapshooters: Vec::new(),
            last_update: Instant::now(),
            max_inactivity_duration: None,
        }
    }

    pub fn set_counter<I: Into<InstrumentAdapter<L, Counter>>>(&mut self, counter: I) {
        self.counter = Some(counter.into());
    }

    pub fn counter<I: Into<InstrumentAdapter<L, Counter>>>(mut self, counter: I) -> Self {
        self.set_counter(counter);
        self
    }

    pub fn get_counter(&self) -> Option<&Counter> {
        self.counter.as_ref().map(|adapter| adapter.instrument())
    }

    pub fn set_gauge<I: Into<GaugeAdapter<L>>>(&mut self, gauge: I) {
        self.gauge = Some(gauge.into());
    }

    pub fn gauge<I: Into<GaugeAdapter<L>>>(mut self, gauge: I) -> Self {
        self.set_gauge(gauge);
        self
    }

    pub fn get_gauge(&self) -> Option<&Gauge> {
        self.gauge.as_ref().map(|adapter| adapter.gauge())
    }

    pub fn set_meter<I: Into<InstrumentAdapter<L, Meter>>>(&mut self, meter: I) {
        self.meter = Some(meter.into());
    }

    pub fn meter<I: Into<InstrumentAdapter<L, Meter>>>(mut self, meter: I) -> Self {
        self.set_meter(meter);
        self
    }

    pub fn get_meter(&self) -> Option<&Meter> {
        self.meter.as_ref().map(|adapter| adapter.instrument())
    }

    pub fn set_histogram<I: Into<InstrumentAdapter<L, Histogram>>>(&mut self, histogram: I) {
        self.histogram = Some(histogram.into());
    }

    pub fn histogram<I: Into<InstrumentAdapter<L, Histogram>>>(mut self, histogram: I) -> Self {
        self.set_histogram(histogram);
        self
    }

    pub fn get_histogram(&self) -> Option<&Histogram> {
        self.histogram.as_ref().map(|adapter| adapter.instrument())
    }

    pub fn add_snapshooter<T: PutsSnapshot>(&mut self, snapshooter: T) {
        self.snapshooters.push(Box::new(snapshooter));
    }

    pub fn snapshooter<T: PutsSnapshot>(mut self, snapshooter: T) -> Self {
        self.add_snapshooter(snapshooter);
        self
    }

    pub fn add_instrument<I: Instrument>(&mut self, instrument: I) {
        self.handlers
            .push(Box::new(InstrumentAdapter::new(instrument)));
    }

    pub fn instrument<T: Instrument>(mut self, instrument: T) -> Self {
        self.add_instrument(instrument);
        self
    }

    pub fn add_panel(&mut self, panel: Panel<L>) {
        self.panels.push(panel);
    }

    pub fn panel(mut self, panel: Panel<L>) -> Self {
        self.add_panel(panel);
        self
    }

    pub fn add_handler<H: HandlesObservations<Label = L>>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    pub fn handler<H: HandlesObservations<Label = L>>(mut self, handler: H) -> Self {
        self.add_handler(handler);
        self
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

    /// Sets the maximum amount of time this panel may be
    /// inactive until no more snapshots are taken
    ///
    /// Default is no inactivity tracking.
    pub fn set_inactivity_limit(&mut self, limit: Duration) {
        self.max_inactivity_duration = Some(limit);
    }

    pub fn accepts_label(&self, label: &L) -> bool {
        self.label_filter.accepts(label)
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
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
        self.panels
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));
        self.snapshooters
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));
        self.handlers
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));
    }
}

impl<L> PutsSnapshot for Panel<L>
where
    L: Clone + Eq + Send + 'static,
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

impl<L> HandlesObservations for Panel<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) -> usize {
        if !self.label_filter.accepts(observation.label()) {
            return 0;
        }

        let mut instruments_updated = 0;

        self.counter
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));
        self.gauge
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));
        self.meter
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));
        self.histogram
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));
        self.panels
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));
        self.handlers
            .iter_mut()
            .for_each(|x| instruments_updated += x.handle_observation(&observation));

        instruments_updated
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

pub(crate) enum LabelFilter<L> {
    AcceptNone,
    AcceptAll,
    One(L),
    Two(L, L),
    Three(L, L, L),
    Four(L, L, L, L),
    Five(L, L, L, L, L),
    Many(Vec<L>),
}

impl<L> LabelFilter<L>
where
    L: PartialEq + Eq,
{
    pub fn new(label: L) -> Self {
        Self::One(label)
    }

    pub fn many(mut labels: Vec<L>) -> Self {
        if labels.is_empty() {
            return LabelFilter::AcceptNone;
        }

        if labels.len() == 1 {
            return LabelFilter::One(labels.pop().unwrap());
        }

        if labels.len() == 2 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            return LabelFilter::Two(b, a);
        }

        if labels.len() == 3 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            return LabelFilter::Three(c, b, a);
        }

        if labels.len() == 4 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            return LabelFilter::Four(d, c, b, a);
        }

        if labels.len() == 5 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            let ee = labels.pop().unwrap();
            return LabelFilter::Five(ee, d, c, b, a);
        }

        LabelFilter::Many(labels)
    }

    pub fn accepts(&self, label: &L) -> bool {
        match self {
            LabelFilter::AcceptNone => false,
            LabelFilter::AcceptAll => true,
            LabelFilter::One(a) => label == a,
            LabelFilter::Two(a, b) => label == a || label == b,
            LabelFilter::Three(a, b, c) => label == a || label == b || label == c,
            LabelFilter::Four(a, b, c, d) => label == a || label == b || label == c || label == d,
            LabelFilter::Five(a, b, c, d, ee) => {
                label == a || label == b || label == c || label == d || label == ee
            }
            LabelFilter::Many(many) => many.contains(label),
        }
    }

    pub fn add_label(&mut self, label: L) {
        let current = std::mem::replace(self, LabelFilter::AcceptNone);
        *self = match current {
            LabelFilter::AcceptAll => LabelFilter::AcceptAll,
            LabelFilter::AcceptNone => LabelFilter::One(label),
            LabelFilter::One(a) => LabelFilter::Two(a, label),
            LabelFilter::Two(a, b) => LabelFilter::Three(a, b, label),
            LabelFilter::Three(a, b, c) => LabelFilter::Four(a, b, c, label),
            LabelFilter::Four(a, b, c, d) => LabelFilter::Five(a, b, c, d, label),
            LabelFilter::Five(a, b, c, d, ee) => {
                let mut labels = vec![a, b, c, d, ee];
                labels.push(label);
                LabelFilter::Many(labels)
            }
            LabelFilter::Many(mut labels) => {
                labels.push(label);
                LabelFilter::Many(labels)
            }
        };
    }
}

impl<L> Default for LabelFilter<L> {
    fn default() -> Self {
        Self::AcceptAll
    }
}

pub struct InstrumentAdapter<L, I> {
    label_filter: LabelFilter<L>,
    instrument: I,
}

impl<L, I> InstrumentAdapter<L, I>
where
    L: Eq,
    I: Instrument,
{
    pub fn new(instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::AcceptAll,
        }
    }

    pub fn for_label(label: L, instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::new(label),
        }
    }

    pub fn for_labels(labels: Vec<L>, instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::many(labels),
        }
    }

    pub fn deaf(instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::AcceptNone,
        }
    }

    pub fn accept_label(mut self, label: L) -> Self {
        self.label_filter.add_label(label);
        self
    }

    pub fn accept_no_labels(mut self) -> Self {
        self.label_filter = LabelFilter::AcceptNone;
        self
    }

    pub fn accept_all_labels(mut self) -> Self {
        self.label_filter = LabelFilter::AcceptAll;
        self
    }

    pub fn instrument(&self) -> &I {
        &self.instrument
    }
}

impl<L, I> HandlesObservations for InstrumentAdapter<L, I>
where
    L: Clone + Eq + Send + 'static,
    I: Instrument,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) -> usize {
        if !self.label_filter.accepts(observation.label()) {
            return 0;
        }

        let BorrowedLabelAndUpdate(_label, update) = observation.into();

        self.instrument.update(&update)
    }
}

impl<L, I> Updates for InstrumentAdapter<L, I>
where
    L: Clone + Eq + Send + 'static,
    I: Instrument,
{
    fn update(&mut self, with: &Update) -> usize {
        self.instrument.update(with)
    }
}

impl<L, I> Instrument for InstrumentAdapter<L, I>
where
    L: Clone + Eq + Send + 'static,
    I: Instrument,
{
}

impl<L, I> PutsSnapshot for InstrumentAdapter<L, I>
where
    L: Send + 'static,
    I: Instrument,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        self.instrument.put_snapshot(into, descriptive)
    }
}

impl<L, I> From<I> for InstrumentAdapter<L, I>
where
    L: Clone + Eq + Send + 'static,
    I: Instrument,
{
    fn from(instrument: I) -> InstrumentAdapter<L, I> {
        InstrumentAdapter::new(instrument)
    }
}

fn duration_to_display_value(time: u64, current_unit: TimeUnit, target_unit: TimeUnit) -> u64 {
    use TimeUnit::*;
    match (current_unit, target_unit) {
        (Nanoseconds, Nanoseconds) => time,
        (Nanoseconds, Microseconds) => time / 1_000,
        (Nanoseconds, Milliseconds) => time / 1_000_000,
        (Nanoseconds, Seconds) => time / 1_000_000_000,
        (Microseconds, Nanoseconds) => time * 1_000,
        (Microseconds, Microseconds) => time,
        (Microseconds, Milliseconds) => time / 1_000,
        (Microseconds, Seconds) => time / 1_000_000,
        (Milliseconds, Nanoseconds) => time * 1_000_000,
        (Milliseconds, Microseconds) => time * 1_000,
        (Milliseconds, Milliseconds) => time,
        (Milliseconds, Seconds) => time / 1_000,
        (Seconds, Nanoseconds) => time * 1_000_000_000,
        (Seconds, Microseconds) => time * 1_000_000,
        (Seconds, Milliseconds) => time * 1_000,
        (Seconds, Seconds) => time,
    }
}
