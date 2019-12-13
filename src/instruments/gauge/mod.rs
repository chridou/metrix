use std::cell::RefCell;
use std::time::Duration;

use crate::instruments::{fundamentals::buckets::SecondsBuckets, Instrument, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, ObservedValue, PutsSnapshot, TimeUnit, DECR, INCR};
pub use gauge_adapter::*;
use tracking::*;

mod gauge_adapter;
mod tracking;

/// Simply returns the value that has been observed last.
///
/// Reacts  `Observation::Observation::ObservedOneValue`
/// with the following values:
/// * `ObservedValue::ChangedBy` whoich increments or decrements the value
/// * All `ObservedValue`s tha can be converted to an `i64` which
/// directly set the value
///
/// # Examples
///
/// ```
/// # use std::time::Instant;
/// # use metrix::instruments::*;
///
/// let mut gauge = Gauge::new_with_defaults("example");
/// assert_eq!(None, gauge.get());
/// let update = Update::ObservationWithValue(12.into(), Instant::now());
/// gauge.update(&update);
///
/// assert_eq!(Some(12), gauge.get());
/// ```
///
/// ```
/// # use std::time::Instant;
/// # use metrix::instruments::*;
/// use metrix::{ChangeBy, Increment, Decrement};
///
/// let mut gauge = Gauge::new_with_defaults("example");
/// gauge.set(12.into());
/// assert_eq!(Some(12), gauge.get());
///
/// let update = Update::ObservationWithValue(Increment.into(), Instant::now());
/// gauge.update(&update);
/// assert_eq!(Some(13), gauge.get());
///
/// let update = Update::ObservationWithValue(Decrement.into(), Instant::now());
/// gauge.update(&update);
/// assert_eq!(Some(12), gauge.get());
///
/// let update = Update::ObservationWithValue(ChangeBy(-12).into(), Instant::now());
/// gauge.update(&update);
/// assert_eq!(Some(0), gauge.get());
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

    #[deprecated(since = "0.10.5", note = "use method `set_tracking`")]
    pub fn set_memorize_extrema(&mut self, d: Duration) {
        self.set_tracking(std::cmp::max(1, d.as_secs() as usize));
    }

    #[deprecated(since = "0.10.5", note = "use method `tracking`")]
    pub fn memorize_extrema(mut self, d: Duration) -> Self {
        self.set_tracking(std::cmp::max(1, d.as_secs() as usize));
        self
    }

    /// Enables tracking of value for the last `for_seconds` seconds.
    /// For each interval of a second there will be a record that tracks
    /// the minimum and maximum values of the gauge within an interval.
    /// Also a sum and a counter to calculate averages will be recorded.
    ///
    /// If tracking is enabled, the following fields will be added:
    ///
    /// * `[gauge_name]_peak`: The peak value of all records
    /// * `[gauge_name]_peak_min`: The smallest of the peak values of all records
    /// * `[gauge_name]_peak_avg`: The average of the peak values for all records
    /// * `[gauge_name]_bottom`: The bottom value of all records
    /// * `[gauge_name]_bottom_max`: The biggest bottom value of all records
    /// * `[gauge_name]_bottom_avg`: The average of all bottom values for all records
    /// * `[gauge_name]_avg`: The average of all values for all records
    pub fn tracking(mut self, for_seconds: usize) -> Self {
        self.set_tracking(for_seconds);
        self
    }

    /// Enables tracking of value for the last `for_seconds` seconds.
    /// For each interval of a second there will be a record that tracks
    /// the minimum and maximum values of the gauge within an interval.
    /// Also a sum and a counter to calculate averages will be recorded.
    ///
    /// If tracking is enabled, the following fields will be added:
    ///
    /// * `[gauge_name]_peak`: The peak value of all records
    /// * `[gauge_name]_peak_min`: The smallest of the peak values of all records
    /// * `[gauge_name]_peak_avg`: The average of the peak values for all records
    /// * `[gauge_name]_bottom`: The bottom value of all records
    /// * `[gauge_name]_bottom_max`: The biggest bottom value of all records
    /// * `[gauge_name]_bottom_avg`: The average of all bottom values for all records
    /// * `[gauge_name]_avg`: The average of all values for all records
    pub fn set_tracking(&mut self, for_seconds: usize) {
        if for_seconds != 0 {
            self.tracking = Some(RefCell::new(SecondsBuckets::new(for_seconds)))
        }
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

    /// Creates a `GaugeAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<L, P>(self, label_predicate: P) -> GaugeAdapter<L>
    where
        L: Eq,
        P: Fn(&L) -> bool + Send + 'static,
    {
        GaugeAdapter::for_labels_by_predicate(label_predicate, self)
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

    pub fn for_labels_deltas_only_by_predicate<L, P>(self, label_predicate: P) -> GaugeAdapter<L>
    where
        L: Eq,
        P: Fn(&L) -> bool + Send + 'static,
    {
        GaugeAdapter::for_labels_deltas_only_by_predicate(label_predicate, self)
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

    pub fn inc_dec_by_predicates<L, PINC, PDEC>(
        self,
        predicate_inc: PINC,
        predicate_dec: PDEC,
    ) -> GaugeAdapter<L>
    where
        L: Eq,
        PINC: Fn(&L) -> bool + Send + 'static,
        PDEC: Fn(&L) -> bool + Send + 'static,
    {
        GaugeAdapter::inc_dec_by_predicates(predicate_inc, predicate_dec, self)
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
        self.value
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
                        .for_each(|stats| stats.add_to_snapshot(into, &self.name)),
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

#[cfg(test)]
mod test;
