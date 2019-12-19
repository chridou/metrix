use crate::snapshot::Snapshot;
use crate::{HandlesObservations, Observation, PutsSnapshot};

use super::*;

pub(crate) enum UpdateModifier<L> {
    KeepAsIs,
    Modify(Box<dyn Fn(&L, Update) -> Update + Send + 'static>),
}

impl<L> UpdateModifier<L> {
    pub fn modify(&self, label: &L, update: Update) -> Update {
        match self {
            UpdateModifier::KeepAsIs => update,
            UpdateModifier::Modify(ref f) => f(label, update),
        }
    }
}

pub struct InstrumentAdapter<L, I> {
    label_filter: LabelFilter<L>,
    instrument: I,
    modify_update: UpdateModifier<L>,
}

impl<L, I> InstrumentAdapter<L, I>
where
    L: Eq + Send + 'static,
    I: Instrument,
{
    pub fn new(instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::accept_all(),
            modify_update: UpdateModifier::KeepAsIs,
        }
    }

    pub fn accept<F: Into<LabelFilter<L>>>(accept: F, instrument: I) -> Self {
        Self {
            instrument,
            label_filter: accept.into(),
            modify_update: UpdateModifier::KeepAsIs,
        }
    }

    pub fn for_label(label: L, instrument: I) -> Self {
        Self::accept(label, instrument)
    }

    pub fn for_labels(labels: Vec<L>, instrument: I) -> Self {
        Self::accept(labels, instrument)
    }

    pub fn by_predicate<P>(predicate: P, instrument: I) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self::accept(LabelPredicate(predicate), instrument)
    }

    pub fn deaf(instrument: I) -> Self {
        Self {
            instrument,
            label_filter: LabelFilter::accept_none(),
            modify_update: UpdateModifier::KeepAsIs,
        }
    }

    pub fn accept_no_labels(mut self) -> Self {
        self.label_filter = LabelFilter::accept_none();
        self
    }

    pub fn accept_all_labels(mut self) -> Self {
        self.label_filter = LabelFilter::accept_all();
        self
    }

    pub fn modify_with<F>(mut self, f: F) -> Self
    where
        F: Fn(&L, Update) -> Update + Send + 'static,
    {
        self.modify_update = UpdateModifier::Modify(Box::new(f));
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

        let BorrowedLabelAndUpdate(label, update) = observation.into();

        let update = self.modify_update.modify(label, update);

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
