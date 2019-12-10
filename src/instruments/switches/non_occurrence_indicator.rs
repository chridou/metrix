use std::time::{Duration, Instant};

use crate::instruments::{Instrument, InstrumentAdapter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, PutsSnapshot};

use super::NameAlternation;

/// Changes the state based on the absence of
/// an observation
/// within a given time.
///
/// Can be used for alerting, e.g. if something
/// expected was not observed within a given time-frame.
///
/// Note:
/// The first occurrence will be when this instrument is
/// created so that the indicator does not turn on
/// right from the start.
pub struct NonOccurrenceIndicator {
    name: String,
    title: Option<String>,
    description: Option<String>,
    if_not_happened_within: Duration,
    happened_last: Instant,
    invert: bool,
    show_inverted: Option<NameAlternation>,
}

impl NonOccurrenceIndicator {
    pub fn new<T: Into<String>>(name: T) -> NonOccurrenceIndicator {
        NonOccurrenceIndicator {
            name: name.into(),
            title: None,
            description: None,
            if_not_happened_within: Duration::from_secs(60),
            happened_last: Instant::now(),
            invert: false,
            show_inverted: None,
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> NonOccurrenceIndicator {
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
    pub fn set_invert_enabled(&mut self, invert: bool) {
        self.invert = invert
    }

    /// Set whether the current value should be inverted in a snapshot or not
    ///
    /// Default is `false`
    pub fn invert_enabled(mut self, invert: bool) -> Self {
        self.set_invert_enabled(invert);
        self
    }

    /// The current value should be inverted in a snapshot
    ///
    /// Same as `self.set_invert(true);`
    pub fn inverted(mut self) -> Self {
        self.set_invert_enabled(true);
        self
    }

    /// return whether invert is on or off
    pub fn is_inverted(&self) -> bool {
        self.invert
    }

    pub fn set_if_not_happened_within(&mut self, d: Duration) {
        self.if_not_happened_within = d;
    }

    pub fn if_not_happened_within(mut self, d: Duration) -> Self {
        self.set_if_not_happened_within(d);
        self
    }

    pub fn get_if_not_happened_within(&self) -> Duration {
        self.if_not_happened_within
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

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations on the given label.
    pub fn for_label<L: Eq>(self, label: L) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::for_label(label, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument
    /// react on observations with the given labels.
    ///
    /// If `labels` is empty the instrument will not react to any observations
    pub fn for_labels<L: Eq>(self, labels: Vec<L>) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::for_labels(labels, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// all observations.
    pub fn for_all_labels<L: Eq>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::new(self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument to no
    /// observations.
    pub fn adapter<L: Eq>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::deaf(self)
    }

    /// Returns the current state
    pub fn state(&self) -> bool {
        let must_have_happened_after = Instant::now() - self.if_not_happened_within;
        let current_state = self.happened_last > must_have_happened_after;

        if self.invert {
            !current_state
        } else {
            current_state
        }
    }
}

impl Instrument for NonOccurrenceIndicator {}

impl PutsSnapshot for NonOccurrenceIndicator {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        into.items.push((self.name.clone(), self.state().into()));
        if let Some(alternation) = &self.show_inverted {
            let label = alternation.adjust_name(&self.name);
            into.items.push((label.into(), (!self.state()).into()));
        }
    }
}

impl Updates for NonOccurrenceIndicator {
    fn update(&mut self, _: &Update) -> usize {
        self.happened_last = Instant::now();
        1
    }
}

impl Descriptive for NonOccurrenceIndicator {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
