use std::cell::RefCell;
use std::time::Duration;

use crate::instruments::{
    fundamentals::buckets::SecondsBuckets, BorrowedLabelAndUpdate, Instrument, LabelFilter, Update,
    Updates,
};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{
    Descriptive, HandlesObservations, Observation, ObservedValue, PutsSnapshot, TimeUnit, DECR,
    INCR,
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
/// let update = Update::ObservationWithValue(12.into(), Instant::now());
/// gauge.update(&update);
///
/// assert_eq!(Some(12), gauge.get());
/// ```
pub struct Gauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<i64>,
    tracking: Option<RefCell<SecondsBuckets<Bucket>>>,
    display_time_unit: TimeUnit,
}

impl Gauge {
    pub fn new<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
            tracking: None,
            display_time_unit: TimeUnit::default(),
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
    #[deprecated(since = "0.10.5", note = "use method `set_enable_tracking`")]
    pub fn set_memorize_extrema(&mut self, d: Duration) {
        self.set_tracking(std::cmp::max(1, d.as_secs() as usize));
    }

    /// If set to `Some(Duration)` a peak and bottom values will
    /// be displayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// If set to None the peak and bottom values will not be shown.
    #[deprecated(since = "0.10.5", note = "use method `enable_tracking`")]
    pub fn memorize_extrema(mut self, d: Duration) -> Self {
        self.set_tracking(std::cmp::max(1, d.as_secs() as usize));
        self
    }

    /// Enables tracking of value for the last `for_seconds` seconds.
    ///
    /// Peak and bottom values will
    /// be displayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// # Panics
    ///
    /// If `for_seconds` is zero.
    pub fn tracking(mut self, for_seconds: usize) -> Self {
        self.set_tracking(for_seconds);
        self
    }

    /// Enables tracking of value for the last `for_seconds` seconds.
    ///
    /// Peak and bottom values will
    /// be displayed for the given duration unless there
    /// is a new peak or bottom which will reset the timer.
    /// The fields has the names `[gauge_name]_peak` and
    /// `[gauge_name]_bottom`
    ///
    /// # Panics
    ///
    /// If `for_seconds` is zero.
    pub fn set_tracking(&mut self, for_seconds: usize) {
        self.tracking = Some(RefCell::new(SecondsBuckets::new(for_seconds)))
    }

    pub fn set_display_time_unit(&mut self, display_time_unit: TimeUnit) {
        self.display_time_unit = display_time_unit
    }

    pub fn display_time_unit(mut self, display_time_unit: TimeUnit) -> Self {
        self.set_display_time_unit(display_time_unit);
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

        if let Some(value) = self.value.take() {
            let next_value = if let Some(next_value) = self.next_value(Some(value), observed) {
                if let Some(ref buckets) = self.tracking {
                    match buckets.try_borrow_mut() {
                        Ok(mut borrowed) => borrowed.current_mut().update(next_value),
                        Err(_err) => crate::util::log_error("borrow mut in gauge::set failed!"),
                    }
                }
                next_value
            } else {
                value
            };

            self.value = Some(next_value);
        } else {
            self.value = self.next_value(None, observed).map(|next_value| {
                if let Some(ref buckets) = self.tracking {
                    match buckets.try_borrow_mut() {
                        Ok(mut borrowed) => borrowed.current_mut().update(next_value),
                        Err(_err) => crate::util::log_error("borrow mut in gauge::set failed!"),
                    }
                }

                next_value
            });
        }
    }

    pub fn get(&self) -> Option<i64> {
        self.value.clone()
    }

    fn next_value(&self, current: Option<i64>, observed: ObservedValue) -> Option<i64> {
        match observed {
            ObservedValue::ChangedBy(d) => current.map(|c| c + d).or_else(|| Some(d)),
            ObservedValue::Duration(time, unit) => {
                let value = super::duration_to_display_value(time, unit, self.display_time_unit);
                Some(value as i64)
            }
            x => x.convert_to_i64().or_else(|| current),
        }
    }
}

impl Instrument for Gauge {}

impl PutsSnapshot for Gauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        if let Some(value) = self.value {
            into.items.push((self.name.clone(), value.into()));
            if let Some(ref buckets) = self.tracking {
                match buckets.try_borrow_mut() {
                    Ok(mut borrowed) => BucketsStats::from_buckets(&mut *borrowed)
                        .into_iter()
                        .for_each(|stats| stats.add_to_snaphot(into, &self.name)),
                    Err(_err) => {
                        crate::util::log_error("borrow mut in gauge::put_snapshot failed!")
                    }
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
    L: Eq + Send + 'static,
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

#[derive(Default)]
struct Bucket {
    pub sum: i64,
    pub count: u64,
    pub min_max: (i64, i64),
}

impl Bucket {
    pub fn update(&mut self, v: i64) {
        self.min_max = if self.count != 0 {
            let (min, max) = self.min_max;
            (std::cmp::min(min, v), std::cmp::max(max, v))
        } else {
            (v, v)
        };
        self.sum += v;
        self.count += 1;
    }
}

struct BucketsStats {
    bottom: i64,
    peak: i64,
    avg: f64,
    bottom_avg: f64,
    peak_avg: f64,
}

impl BucketsStats {
    fn from_buckets(buckets: &mut SecondsBuckets<Bucket>) -> Option<Self> {
        let mut bottom = std::i64::MAX;
        let mut peak = std::i64::MIN;
        let mut sum_bottom = 0;
        let mut sum_peak = 0;
        let mut total_sum = 0;
        let mut total_count = 0;

        buckets.iter().for_each(
            |Bucket {
                 sum,
                 count,
                 min_max,
             }| {
                total_sum += sum;
                total_count += count;

                if *count != 0 {
                    let (min, max) = min_max;
                    bottom = std::cmp::min(bottom, *min);
                    peak = std::cmp::max(peak, *max);
                    sum_bottom += min;
                    sum_peak += max;
                }
            },
        );

        if total_count > 0 {
            let avg = (total_sum as f64) / (total_count as f64);
            let bottom_avg = (sum_bottom as f64) / (buckets.len() as f64);
            let peak_avg = (sum_peak as f64) / (buckets.len() as f64);
            Some(BucketsStats {
                bottom,
                peak,
                avg,
                bottom_avg,
                peak_avg,
            })
        } else {
            None
        }
    }

    pub fn add_to_snaphot(self, snapshot: &mut Snapshot, name: &str) {
        let bottom_name = format!("{}_bottom", name);
        let peak_name = format!("{}_peak", name);
        let avg_name = format!("{}_avg", name);
        let bottom_avg_name = format!("{}_bottom_avg", name);
        let peak_avg_name = format!("{}_peak_avg", name);

        snapshot.items.push((bottom_name, self.bottom.into()));
        snapshot.items.push((peak_name, self.peak.into()));
        snapshot.items.push((avg_name, self.avg.into()));
        snapshot
            .items
            .push((bottom_avg_name, self.bottom_avg.into()));
        snapshot.items.push((peak_avg_name, self.peak_avg.into()));
    }
}

#[cfg(test)]
mod test;
