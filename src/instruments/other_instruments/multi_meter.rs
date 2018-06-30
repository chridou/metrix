use std::time::{Duration, Instant};

use instruments::meter::{MeterRate, MeterSnapshot};
use instruments::{BorrowedLabelAndUpdate, Instrument, Meter, Update, Updates};
use snapshot::{ItemKind, Snapshot};
use util;
use {Descriptive, HandlesObservations, Observation, PutsSnapshot};

pub struct MultiMeter<L> {
    name: String,
    title: Option<String>,
    description: Option<String>,
    meters: Vec<(L, Meter)>,
}

impl<L> MultiMeter<L>
where
    L: Eq + Send + 'static,
{
    pub fn new_with_defaults<T: Into<String>>(name: T) -> MultiMeter<L> {
        MultiMeter {
            name: name.into(),
            title: None,
            description: None,
            meters: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn add_meter(&mut self, label: L, meter: Meter) {
        self.meters.push((label, meter))
    }
}

impl<L> PutsSnapshot for MultiMeter<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        // TBD
    }
}

impl<L> HandlesObservations for MultiMeter<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) {
        let BorrowedLabelAndUpdate(incoming_label, update) = observation.into();

        self.meters
            .iter_mut()
            .filter(|(label, _)| incoming_label == label)
            .for_each(|(_, meter)| meter.update(&update));
    }
}

impl<L> ::Descriptive for MultiMeter<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
