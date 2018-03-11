use instruments::{Instrument, Update, Updates};

use snapshot::Snapshot;
use {Descriptive, PutsSnapshot};
use util;

/// A simple ever increasing counter
///
/// Reacts to the following `Observation`s:
///
/// * `Observation::Observed`(Update::Observations)
/// * `Obervation::ObservedOne`(Update::Observation)
/// * `Obervation::ObservedOneValue`(Update::ObservationWithValue)
///
/// # Example
///
/// ```
/// use std::time::Instant;
/// use metrix::instruments::*;
///
/// let mut counter = Counter::new_with_defaults("example");
/// let update = Update::Observation(Instant::now());
/// counter.update(&update);
///
/// assert_eq!(1, counter.get());
/// ```
pub struct Counter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    count: u64,
}

impl Counter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
            title: None,
            description: None,
            count: 0,
        }
    }

    /// Increase the stored value by one.
    pub fn inc(&mut self) {
        self.count += 1;
    }

    /// Increase the stored value by `n`
    pub fn inc_by(&mut self, n: u64) {
        self.count += n;
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.count
    }

    /// Gets the name of this `Counter`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `Counter`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `Counter`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `Counter`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }
}

impl Instrument for Counter {}

impl PutsSnapshot for Counter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        into.items.push((self.name.clone(), self.count.into()));
    }
}

impl Updates for Counter {
    fn update(&mut self, with: &Update) {
        match *with {
            Update::Observation(_) => self.inc(),
            Update::Observations(n, _) => self.inc_by(n),
            Update::ObservationWithValue(_, _) => self.inc(),
        }
    }
}

impl Descriptive for Counter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
