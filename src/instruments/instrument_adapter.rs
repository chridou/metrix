use crate::snapshot::Snapshot;
use crate::{HandlesObservations, Observation, PutsSnapshot};

use super::*;

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
    L: Eq + Send + 'static,
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
