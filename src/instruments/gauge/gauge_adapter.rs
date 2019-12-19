use crate::instruments::{
    AcceptAllLabels, BorrowedLabelAndUpdate, LabelFilter, LabelPredicate, Update, UpdateModifier,
    Updates,
};
use crate::snapshot::Snapshot;
use crate::{HandlesObservations, Observation, ObservedValue, PutsSnapshot};

use super::*;

pub struct GaugeAdapter<L> {
    strategy: GaugeUpdateStrategy<L>,
    gauge: Gauge,
    modify_update: UpdateModifier<L>,
}

impl<L> GaugeAdapter<L>
where
    L: Eq + Send + 'static,
{
    /// Creates a new adapter which dispatches all observations
    /// to the `Gauge`
    pub fn new(gauge: Gauge) -> Self {
        Self::accept(AcceptAllLabels, gauge)
    }

    pub fn accept<F: Into<LabelFilter<L>>>(accept: F, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(accept.into()),
            modify_update: UpdateModifier::KeepAsIs,
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given label to the `Gauge`
    pub fn for_label(label: L, gauge: Gauge) -> Self {
        Self::accept(label, gauge)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge`
    ///
    /// If `labels` is empty, no observations will be dispatched
    pub fn for_labels(labels: Vec<L>, gauge: Gauge) -> Self {
        Self::accept(labels, gauge)
    }

    /// Creates a `GaugeAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<P>(label_predicate: P, gauge: Gauge) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self::accept(LabelPredicate(label_predicate), gauge)
    }

    pub fn deltas_only<F: Into<LabelFilter<L>>>(accept: F, gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::DeltasOnly(accept.into()),
            modify_update: UpdateModifier::KeepAsIs,
        }
    }

    /// Creates a new adapter which dispatches observations
    /// with the given label to the `Gauge`
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_label_deltas_only(label: L, gauge: Gauge) -> Self {
        Self::deltas_only(label, gauge)
    }

    /// Creates a new adapter which dispatches observations
    /// with the given labels to the `Gauge`
    ///
    /// If `labels` is empty, no observations will be dispatched
    ///
    /// The Gauge will only be dispatched message that increment or
    /// decrement the value
    pub fn for_labels_deltas_only(labels: Vec<L>, gauge: Gauge) -> Self {
        Self::deltas_only(labels, gauge)
    }

    /// Creates a `GaugeAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_deltas_only_by_predicate<P>(label_predicate: P, gauge: Gauge) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self::deltas_only(LabelPredicate(label_predicate), gauge)
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
    pub fn inc_dec_on<INCR: Into<LabelFilter<L>>, DECR: Into<LabelFilter<L>>>(
        accept_incr: INCR,
        accept_decr: DECR,
        gauge: Gauge,
    ) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::IncDecOnLabels(accept_incr.into(), accept_decr.into()),
            modify_update: UpdateModifier::KeepAsIs,
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
        Self::inc_dec_on(increment_on, decrement_on, gauge)
    }

    /// Creates a `GaugeAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn inc_dec_by_predicates<PINC, PDEC>(
        predicate_inc: PINC,
        predicate_dec: PDEC,
        gauge: Gauge,
    ) -> Self
    where
        PINC: Fn(&L) -> bool + Send + 'static,
        PDEC: Fn(&L) -> bool + Send + 'static,
    {
        Self::inc_dec_on(
            LabelPredicate(predicate_inc),
            LabelPredicate(predicate_dec),
            gauge,
        )
    }

    /// Creates a new adapter which dispatches **no**
    /// observations to the `Gauge`
    pub fn deaf(gauge: Gauge) -> Self {
        Self {
            gauge,
            strategy: GaugeUpdateStrategy::Filter(LabelFilter::accept_none()),
            modify_update: UpdateModifier::KeepAsIs,
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
                let update = self.modify_update.modify(label, update);
                self.gauge.update(&update)
            }
            GaugeUpdateStrategy::DeltasOnly(ref filter) => {
                if !filter.accepts(label) {
                    return 0;
                }
                let update = self.modify_update.modify(label, update);

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
