use std::time::{Duration, Instant};

use crate::instruments::{Instrument, Update, Updates};
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
    pub fn new_with_defaults<T: Into<String>>(name: T) -> NonOccurrenceIndicator {
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

    /// Gets the name of this `NonOccurenceIndicator`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `NonOccurenceIndicator`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `NonOccurenceIndicator`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `NonOccurenceIndicator`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
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

    pub fn set_if_not_happened_within(&mut self, d: Duration) {
        self.if_not_happened_within = d;
    }

    pub fn if_not_happened_within(&self) -> Duration {
        self.if_not_happened_within
    }

    /// Show the inverted value. Name will be adjusted with `name_alternation`.
    pub fn show_inverted(&mut self, name_alternation: NameAlternation) {
        self.show_inverted = Some(name_alternation)
    }

    /// Show the inverted value. Name will be prefixed with `prefix`.
    pub fn show_inverted_prefixed<T: Into<String>>(&mut self, prefix: T) {
        self.show_inverted(NameAlternation::Prefix(prefix.into()))
    }

    /// Show the inverted value. Name will be postfixed with `postfix`.
    pub fn show_inverted_postfixed<T: Into<String>>(&mut self, postfix: T) {
        self.show_inverted(NameAlternation::Postfix(postfix.into()))
    }

    /// Show the inverted value. Name will be renamed with `new_name`.
    pub fn show_inverted_renamed<T: Into<String>>(&mut self, new_name: T) {
        self.show_inverted(NameAlternation::Rename(new_name.into()))
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
