use std::time::{Duration, Instant};

use crate::instruments::{BorrowedLabelAndUpdate, Instrument, LabelFilter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{
    Descriptive, HandlesObservations, Observation, ObservedValue, PutsSnapshot, DECR, INCR,
};

/// Simply returns the value that has been observed last.
///
/// Reacts to the following `Observation`:
///
/// * `Observation::ObservedOneValue`(Update::ObservationWithValue)
///
/// # Example
///
/// ```
/// use std::time::Instant;
/// use metrix::instruments::*;
///
/// let mut gauge = Gauge::new_with_defaults("example");
/// assert_eq!(None, gauge.get());
/// let update = Update::ObservationWithValue(12, Instant::now());
/// gauge.update(&update);
///
/// assert_eq!(Some(12), gauge.get());
/// ```
pub struct Gauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<State>,
    memorize_extrema: Option<Duration>,
}

impl Gauge {
    pub fn new<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
            memorize_extrema: None,
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
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

    /// If set to `Some(Duration)` a peak and bottom values will
    /// be displayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// If set to None the peak and bottom values will not be shown.
    pub fn set_memorize_extrema(&mut self, d: Duration) {
        self.memorize_extrema = Some(d)
    }

    /// If set to `Some(Duration)` a peak and bottom values will
    /// be displayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// If set to None the peak and bottom values will not be shown.
    pub fn memorize_extrema(mut self, d: Duration) -> Self {
        self.set_memorize_extrema(d);
        self
    }

    /// Creates an `GaugeAdapter` that makes this instrument
    /// react on observations on the given label.
    pub fn for_label<L: Eq>(self, label: L) -> GaugeAdapter<L> {
        GaugeAdapter::for_label(label, self)
    }

    /// Creates an `GaugeAdapter` that makes this instrument
    /// react on observations with the given labels.
    ///
    /// If `labels` is empty the instrument will not react to any observations
    pub fn for_labels<L: Eq>(self, label: Vec<L>) -> GaugeAdapter<L> {
        GaugeAdapter::for_labels(label, self)
    }

    /// Creates an `GaugeAdapter` that makes this instrument react on
    /// all observations.
    pub fn for_all_labels<L: Eq>(self) -> GaugeAdapter<L> {
        GaugeAdapter::new(self)
    }

    /// Creates an `GaugeAdapter` that makes this instrument to rect to no
    /// observations.
    pub fn adapter<L: Eq>(self) -> GaugeAdapter<L> {
        GaugeAdapter::deaf(self)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given label to the `Gauge`
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_label_deltas_only<L: Eq>(self, label: L) -> GaugeAdapter<L> {
        GaugeAdapter::for_label_deltas_only(label, self)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge`
    ///
    /// If `labels` is empty, no observations will be dispatched
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_labels_deltas_only<L: Eq>(self, labels: Vec<L>) -> GaugeAdapter<L> {
        GaugeAdapter::for_labels_deltas_only(labels, self)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge` which only increment or
    /// decrement the `Gauge`
    ///
    /// The `Gauge` will increment on any observation with label `increment_on`
    /// and decrement for any observation with label `decrement_on`.
    ///
    /// `increment_on` is evaluated first so
    /// `increment_on` and `decrement_on` should not be the same label.
    pub fn inc_dec_on<L: Eq>(self, increment_on: L, decrement_on: L) -> GaugeAdapter<L> {
        GaugeAdapter::inc_dec_on(increment_on, decrement_on, self)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge` which only increment or
    /// decrement the `Gauge`
    ///
    /// The `Gauge` will increment on any observation with a label in `increment_on`
    /// and decrement for any observation with a label in `decrement_on`.
    ///
    /// `increment_on` is evaluated first so
    /// `increment_on` and `decrement_on` should share labels.
    pub fn inc_dec_on_many<L: Eq>(
        self,
        increment_on: Vec<L>,
        decrement_on: Vec<L>,
    ) -> GaugeAdapter<L> {
        GaugeAdapter::inc_dec_on_many(increment_on, decrement_on, self)
    }

    pub fn set(&mut self, observed: ObservedValue) {
        // the old hack first...
        let observed = if let Some(v) = observed.convert_to_u64() {
            if v == INCR {
                ObservedValue::ChangedBy(1)
            } else if v == DECR {
                ObservedValue::ChangedBy(-1)
            } else {
                observed
            }
        } else {
            observed
        };

        if let Some(mut state) = self.value.take() {
            let next_value = if let Some(next_value) = next_value(Some(state.current), observed) {
                next_value
            } else {
                state.current
            };

            if let Some(ext_dur) = self.memorize_extrema {
                let now = Instant::now();
                if next_value >= state.peak {
                    state.last_peak_at = now;
                    state.peak = next_value;
                } else if state.last_peak_at < now - ext_dur {
                    state.peak = next_value;
                }

                if next_value <= state.bottom {
                    state.last_bottom_at = now;
                    state.bottom = next_value;
                } else if state.last_bottom_at < now - ext_dur {
                    state.bottom = next_value;
                }
            }
            state.current = next_value;
            self.value = Some(state);
        } else {
            self.value = next_value(None, observed).map(|next_value| {
                let now = Instant::now();
                State {
                    current: next_value,
                    peak: next_value,
                    bottom: next_value,
                    last_peak_at: now,
                    last_bottom_at: now,
                }
            });
        }
    }

    pub fn get(&self) -> Option<i64> {
        self.value.as_ref().map(|v| v.current)
    }
}

fn next_value(current: Option<i64>, observed: ObservedValue) -> Option<i64> {
    match observed {
        ObservedValue::ChangedBy(d) => current.map(|c| c + d),
        x => x.convert_to_i64().or_else(|| current),
    }
}

impl Instrument for Gauge {}

impl PutsSnapshot for Gauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        if let Some(ref state) = self.value {
            into.items.push((self.name.clone(), state.current.into()));
            if let Some(ext_dur) = self.memorize_extrema {
                let peak_name = format!("{}_peak", self.name);
                if state.last_peak_at > Instant::now() - ext_dur {
                    into.items.push((peak_name, state.peak.into()));
                } else {
                    into.items.push((peak_name, state.current.into()));
                }
                let bottom_name = format!("{}_bottom", self.name);
                if state.last_bottom_at > Instant::now() - ext_dur {
                    into.items.push((bottom_name, state.bottom.into()));
                } else {
                    into.items.push((bottom_name, state.current.into()));
                }
            }
        }
    }
}

impl Updates for Gauge {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(v, _) => {
                self.set(v);
                1
            }
            _ => 0,
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

struct State {
    current: i64,
    peak: i64,
    bottom: i64,
    last_peak_at: Instant,
    last_bottom_at: Instant,
}

pub struct GaugeAdapter<L> {
    strategy: GaugeUpdateStrategy<L>,
    gauge: Gauge,
}

impl<L> GaugeAdapter<L>
where
    L: Eq,
{
    /// Creates a new adapter which dispatches all observations
    /// to the `Gauge`
    pub fn new(gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(LabelFilter::AcceptAll),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given label to the `Gauge`
    pub fn for_label(label: L, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(LabelFilter::new(label)),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge`
    ///
    /// If `labels` is empty, no observations will be dispatched
    pub fn for_labels(labels: Vec<L>, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(LabelFilter::many(labels)),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given label to the `Gauge`
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_label_deltas_only(label: L, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::DeltasOnly(LabelFilter::new(label)),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge`
    ///
    /// If `labels` is empty, no observations will be dispatched
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_labels_deltas_only(labels: Vec<L>, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::DeltasOnly(LabelFilter::many(labels)),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge` which only increment or
    /// decrement the `Gauge`
    ///
    /// The `Gauge` will increment on any observation with label `increment_on`
    /// and decrement for any observation with label `decrement_on`.
    ///
    /// `increment_on` is evaluated first so
    /// `increment_on` and `decrement_on` should not be the same label.
    pub fn inc_dec_on(increment_on: L, decrement_on: L, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::IncDecOnLabels(
                LabelFilter::new(increment_on),
                LabelFilter::new(decrement_on),
            ),
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge` which only increment or
    /// decrement the `Gauge`
    ///
    /// The `Gauge` will increment on any observation with a label in `increment_on`
    /// and decrement for any observation with a label in `decrement_on`.
    ///
    /// `increment_on` is evaluated first so
    /// `increment_on` and `decrement_on` should share labels.
    pub fn inc_dec_on_many(increment_on: Vec<L>, decrement_on: Vec<L>, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::IncDecOnLabels(
                LabelFilter::many(increment_on),
                LabelFilter::many(decrement_on),
            ),
        }
    }

    /// Creates a new adapter which dispatches **no**
    /// observations to the `Gauge`
    pub fn deaf(gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(LabelFilter::AcceptNone),
        }
    }

    pub fn gauge(&self) -> &Gauge {
        &self.gauge
    }

    pub fn gauge_mut(&mut self) -> &mut Gauge {
        &mut self.gauge
    }

    pub fn into_inner(self) -> Gauge {
        self.gauge
    }
}

impl<L> HandlesObservations for GaugeAdapter<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) -> usize {
        let BorrowedLabelAndUpdate(label, update) = observation.into();

        match self.strategy {
            GaugeUpdateStrategy::Filter(ref filter) => {
                if !filter.accepts(label) {
                    return 0;
                }
                self.gauge.update(&update)
            }
            GaugeUpdateStrategy::DeltasOnly(ref filter) => {
                if !filter.accepts(label) {
                    return 0;
                }

                match update {
                    Update::ObservationWithValue(ObservedValue::ChangedBy(_), _) => {
                        self.gauge.update(&update)
                    }
                    _ => 0,
                }
            }
            GaugeUpdateStrategy::IncDecOnLabels(ref inc, ref dec) => {
                let timestamp = observation.timestamp();
                if inc.accepts(label) {
                    self.gauge.update(&Update::ObservationWithValue(
                        ObservedValue::ChangedBy(1),
                        timestamp,
                    ))
                } else if dec.accepts(label) {
                    self.gauge.update(&Update::ObservationWithValue(
                        ObservedValue::ChangedBy(-1),
                        timestamp,
                    ))
                } else {
                    0
                }
            }
        }
    }
}

impl<L> Updates for GaugeAdapter<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn update(&mut self, with: &Update) -> usize {
        self.gauge.update(with)
    }
}

impl<L> Instrument for GaugeAdapter<L> where L: Clone + Eq + Send + 'static {}

impl<L> PutsSnapshot for GaugeAdapter<L>
where
    L: Send + 'static,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        self.gauge.put_snapshot(into, descriptive)
    }
}

impl<L> From<Gauge> for GaugeAdapter<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn from(gauge: Gauge) -> GaugeAdapter<L> {
        GaugeAdapter::new(gauge)
    }
}

enum GaugeUpdateStrategy<L> {
    Filter(LabelFilter<L>),
    DeltasOnly(LabelFilter<L>),
    IncDecOnLabels(LabelFilter<L>, LabelFilter<L>),
}
