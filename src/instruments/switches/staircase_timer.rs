use std::time::{Duration, Instant};

use crate::instruments::{
    AcceptAllLabels, Instrument, InstrumentAdapter, LabelFilter, LabelPredicate, Update, Updates,
};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, PutsSnapshot};

use super::NameAlternation;

/// A `StaircaseTimer` is 'tapped' by an `Observation`
/// and then stays on for some time.
///
///
/// The `StaircaseTimer` works exactly like a switch in
/// a staircase: You press the switch and the light will
/// stay on for some time. Here it is an `Observation` that
/// pushes the switch and the light is a boolean that will
/// be set to `true` for some time. When triggered while
/// already on, the time being `true` will be prolonged.
///
/// The state written to a `Snapshot` can be inverted.
pub struct StaircaseTimer {
    name: String,
    title: Option<String>,
    description: Option<String>,
    switch_off_after: Duration,
    invert: bool,
    stay_on_until: Option<Instant>,
    show_inverted: Option<NameAlternation>,
}

impl StaircaseTimer {
    pub fn new<T: Into<String>>(name: T) -> StaircaseTimer {
        StaircaseTimer {
            name: name.into(),
            title: None,
            description: None,
            switch_off_after: Duration::from_secs(60),
            invert: false,
            stay_on_until: None,
            show_inverted: None,
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> StaircaseTimer {
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
    /// Set whether the current value should be inverted in a snapshot or not
    ///
    /// Default is `false`
    pub fn set_invert(&mut self, invert: bool) {
        self.invert = invert
    }

    /// The current value should be inverted in a snapshot
    ///
    /// Same as `self.set_invert(true);`
    pub fn enable_invert(&mut self) {
        self.invert = true
    }

    /// return whether invert is on or off
    pub fn invert(&self) -> bool {
        self.invert
    }

    /// Sets duration after which the internal state switches back to `false`
    ///
    /// Default is 60 seconds
    pub fn set_switch_off_after(&mut self, d: Duration) {
        self.switch_off_after = d;
    }

    /// Sets duration after which the internal state switches back to `false`
    ///
    /// Default is 60 seconds
    pub fn switch_off_after(mut self, d: Duration) -> Self {
        self.set_switch_off_after(d);
        self
    }

    /// Gets duration after which the internal state switches back to `false`
    pub fn get_switch_off_after(&self) -> Duration {
        self.switch_off_after
    }

    /// Show the inverted value. Name will be adjusted with `name_alternation`.
    pub fn set_show_inverted(&mut self, name_alternation: NameAlternation) {
        self.show_inverted = Some(name_alternation)
    }

    /// Show the inverted value. Name will be adjusted with `name_alternation`.
    pub fn show_inverted(mut self, name_alternation: NameAlternation) -> Self {
        self.set_show_inverted(name_alternation);
        self
    }

    /// Show the inverted value. Name will be prefixed with `prefix`.
    pub fn set_show_inverted_prefixed<T: Into<String>>(&mut self, prefix: T) {
        self.set_show_inverted(NameAlternation::Prefix(prefix.into()))
    }

    /// Show the inverted value. Name will be prefixed with `prefix`.
    pub fn show_inverted_prefixed<T: Into<String>>(mut self, prefix: T) -> Self {
        self.set_show_inverted(NameAlternation::Prefix(prefix.into()));
        self
    }

    /// Show the inverted value. Name will be postfixed with `postfix`.
    pub fn set_show_inverted_postfixed<T: Into<String>>(&mut self, postfix: T) {
        self.set_show_inverted(NameAlternation::Postfix(postfix.into()))
    }

    /// Show the inverted value. Name will be postfixed with `postfix`.
    pub fn show_inverted_postfixed<T: Into<String>>(mut self, postfix: T) -> Self {
        self.set_show_inverted(NameAlternation::Postfix(postfix.into()));
        self
    }

    /// Show the inverted value. Name will be renamed with `new_name`.
    pub fn set_show_inverted_renamed<T: Into<String>>(&mut self, new_name: T) {
        self.set_show_inverted(NameAlternation::Rename(new_name.into()))
    }

    /// Show the inverted value. Name will be renamed with `new_name`.
    pub fn show_inverted_renamed<T: Into<String>>(mut self, new_name: T) -> Self {
        self.set_show_inverted(NameAlternation::Rename(new_name.into()));
        self
    }

    pub fn accept<L: Eq + Send + 'static, F: Into<LabelFilter<L>>>(
        self,
        accept: F,
    ) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::accept(accept, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations on the given label.
    pub fn for_label<L: Eq + Send + 'static>(self, label: L) -> InstrumentAdapter<L, Self> {
        self.accept(label)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations with the given labels.
    ///
    /// If `labels` is empty the instrument will not react to any observations
    pub fn for_labels<L: Eq + Send + 'static>(self, labels: Vec<L>) -> InstrumentAdapter<L, Self> {
        self.accept(labels)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// all observations.
    pub fn for_all_labels<L: Eq + Send + 'static>(self) -> InstrumentAdapter<L, Self> {
        self.accept(AcceptAllLabels)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<L, P>(self, label_predicate: P) -> InstrumentAdapter<L, Self>
    where
        L: Eq + Send + 'static,
        P: Fn(&L) -> bool + Send + 'static,
    {
        self.accept(LabelPredicate(label_predicate))
    }

    /// Creates an `InstrumentAdapter` that makes this instrument to no
    /// observations.
    pub fn adapter<L: Eq + Send + 'static>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::deaf(self)
    }

    /// Returns the current state
    pub fn state(&self) -> bool {
        let value = if let Some(stay_on_until) = self.stay_on_until {
            stay_on_until >= Instant::now()
        } else {
            false
        };

        if self.invert {
            !value
        } else {
            value
        }
    }
}

impl Instrument for StaircaseTimer {}

impl PutsSnapshot for StaircaseTimer {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        into.items.push((self.name.clone(), self.state().into()));
        if let Some(alternation) = &self.show_inverted {
            let label = alternation.adjust_name(&self.name);
            into.items.push((label.into(), (!self.state()).into()));
        }
    }
}

impl Updates for StaircaseTimer {
    fn update(&mut self, _: &Update) -> usize {
        self.stay_on_until = Some(Instant::now() + self.switch_off_after);
        1
    }
}

impl Descriptive for StaircaseTimer {
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
