use std::time::Instant;

use crate::instruments::{Instrument, InstrumentAdapter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, PutsSnapshot};

/// Tracks how many seconds elapsed since the last occurrence
pub struct LastOccurrenceTracker {
    name: String,
    title: Option<String>,
    description: Option<String>,
    happened_last: Option<Instant>,
    invert: bool,
    make_none_zero: bool,
}

impl LastOccurrenceTracker {
    pub fn new<T: Into<String>>(name: T) -> LastOccurrenceTracker {
        LastOccurrenceTracker {
            name: name.into(),
            title: None,
            description: None,
            happened_last: None,
            invert: false,
            make_none_zero: false,
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> LastOccurrenceTracker {
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

    /// If set to `true` possible `None`s that would
    /// be returned will instead be `0`.
    ///
    /// Hint: This instrument will return `None` unless there
    /// was at least one occurrence recorded.
    pub fn set_make_none_zero(&mut self, make_zero: bool) {
        self.make_none_zero = make_zero
    }

    /// If set to `true` possible `None`s that would
    /// be returned will instead be `0`.
    ///
    /// Hint: This instrument will return `None` unless there
    /// was at least one occurrence recorded.
    pub fn make_none_zero(mut self, make_zero: bool) -> Self {
        self.set_make_none_zero(make_zero);
        self
    }

    /// return whether `make_none_zero` is on or off
    pub fn get_make_none_zero(&self) -> bool {
        self.make_none_zero
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
    pub fn for_labels<L: Eq>(self, label: Vec<L>) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::for_labels(label, self)
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

    fn elapsed_since_last_occurrence(&self) -> Option<u64> {
        self.happened_last
            .map(|last| (Instant::now() - last).as_secs())
    }
}

impl Instrument for LastOccurrenceTracker {}

impl PutsSnapshot for LastOccurrenceTracker {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        if let Some(v) = self.elapsed_since_last_occurrence() {
            into.items.push((self.name.clone(), v.into()));
        } else {
            if self.get_make_none_zero() {
                into.items.push((self.name.clone(), 0.into()));
            }
        }
    }
}

impl Updates for LastOccurrenceTracker {
    fn update(&mut self, _: &Update) -> usize {
        self.happened_last = Some(Instant::now());
        1
    }
}

impl Descriptive for LastOccurrenceTracker {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
