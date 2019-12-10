use crate::instruments::{Instrument, InstrumentAdapter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, PutsSnapshot};

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
    pub fn new<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
            title: None,
            description: None,
            count: 0,
        }
    }
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
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
}

impl Instrument for Counter {}

impl PutsSnapshot for Counter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        into.items.push((self.name.clone(), self.count.into()));
    }
}

impl Updates for Counter {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::Observation(_) => {
                self.inc();
                1
            }
            Update::Observations(n, _) => {
                self.inc_by(n);
                1
            }
            Update::ObservationWithValue(_, _) => {
                self.inc();
                1
            }
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

#[cfg(test)]
mod test {
    use std::time::Instant;

    use super::*;

    #[test]
    fn updates() {
        let mut counter = Counter::new("");

        assert_eq!(counter.get(), 0);

        counter.update(&Update::Observation(Instant::now()));
        assert_eq!(counter.get(), 1);

        counter.update(&Update::Observations(1, Instant::now()));
        assert_eq!(counter.get(), 2);

        counter.update(&Update::Observations(3, Instant::now()));
        assert_eq!(counter.get(), 5);

        counter.update(&Update::ObservationWithValue(33.into(), Instant::now()));
        assert_eq!(counter.get(), 6);
    }
}
