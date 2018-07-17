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
    lower_cutoff: f64,
    one_minute_rate_enabled: bool,
    five_minute_rate_enabled: bool,
    fifteen_minute_rate_enabled: bool,

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
            lower_cutoff: 0.001,
            one_minute_rate_enabled: true,
            five_minute_rate_enabled: false,
            fifteen_minute_rate_enabled: false,
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

    /// Enable tracking of one minute rates.
    ///
    /// Default: enabled
    pub fn set_enable_one_minute_rate_enabled(&mut self, enabled: bool) {
        self.one_minute_rate_enabled = enabled;
        self.meters
            .iter_mut()
            .for_each(|(_, m)| m.set_enable_one_minute_rate_enabled(enabled));
    }

    /// Enable tracking of five minute rates.
    ///
    /// Default: disabled
    pub fn set_five_minute_rate_enabled(&mut self, enabled: bool) {
        self.five_minute_rate_enabled = enabled;
        self.meters
            .iter_mut()
            .for_each(|(_, m)| m.set_five_minute_rate_enabled(enabled));
    }

    /// Enable tracking of one minute rates.
    ///
    /// Default: disabled
    pub fn set_fifteen_minute_rate_enabled(&mut self, enabled: bool) {
        self.fifteen_minute_rate_enabled = enabled;
        self.meters
            .iter_mut()
            .for_each(|(_, m)| m.set_fifteen_minute_rate_enabled(enabled));
    }

    /// Rates below this value will be shown as zero.
    ///
    /// Default is 0.001
    pub fn set_lower_cutoff(&mut self, cutoff: f64) {
        self.lower_cutoff = cutoff;
        self.meters
            .iter_mut()
            .for_each(|(_, m)| m.set_lower_cutoff(cutoff));
    }

    pub fn add_meter<N: Into<String>, T: Into<String>, D: Into<String>>(
        &mut self,
        label: L,
        name: N,
        title: Option<T>,
        description: Option<D>,
    ) {
        let mut meter = Meter::new_with_defaults(name);
        title.into_iter().for_each(|t| meter.set_title(t));
        description
            .into_iter()
            .for_each(|d| meter.set_description(d));
        meter.set_enable_one_minute_rate_enabled(self.one_minute_rate_enabled);
        meter.set_five_minute_rate_enabled(self.five_minute_rate_enabled);
        meter.set_fifteen_minute_rate_enabled(self.fifteen_minute_rate_enabled);
        meter.set_lower_cutoff(self.lower_cutoff);
        self.meters.push((label, meter))
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        let _snapshots: Vec<_> = self.meters.iter().map(|(_, m)| m.get_snapshot()).collect();
    }
}

impl<L> PutsSnapshot for MultiMeter<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level, descriptive);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
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
