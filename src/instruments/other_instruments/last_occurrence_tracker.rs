use std::time::Instant;

use instruments::{Instrument, Update, Updates};
use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot};

/// Tracks how many seconds elapsed since the last occurence
pub struct LastOccurrenceTracker {
    name: String,
    title: Option<String>,
    description: Option<String>,
    happened_last: Option<Instant>,
    invert: bool,
    make_none_zero: bool,
}

impl LastOccurrenceTracker {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> LastOccurrenceTracker {
        LastOccurrenceTracker {
            name: name.into(),
            title: None,
            description: None,
            happened_last: None,
            invert: false,
            make_none_zero: false,
        }
    }

    /// Gets the name of this `OccurenceTracker`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `OccurenceTracker`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `OccurenceTracker`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `OccurenceTracker`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    /// Set whether the current value should be inverted in a snapshot or
    /// not
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

    /// If set to `true` possible `None`s that would
    /// be returned will instead be `0`.
    ///
    /// Hint: This instrument will return `None` unless there
    /// was at least one Occurence recorded.
    pub fn set_make_none_zero(&mut self, make_zero: bool) {
        self.make_none_zero = make_zero
    }

    /// return whether `make_none_zero` is on or off
    pub fn make_none_zero(&self) -> bool {
        self.make_none_zero
    }

    /// Returns the current state
    pub fn elapsed_since_last_occurrence(&self) -> Option<u64> {
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
            if self.make_none_zero() {
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
