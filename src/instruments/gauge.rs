use instruments::{Instrument, Update, Updates};

use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot};

/// Simply returns the value that has been observed last.
///
/// Reacts to the following `Observation`:
///
/// * `Obervation::ObservedOneValue`(Update::ObservationWithValue)
///
/// # Example
///
/// ```
/// use std::time::Instant;
/// use metrix::instruments::*;
///
/// let mut gauge = Gauge::new_with_defaults("example");
/// assert_eq!(None, gauge.get());
/// let update = Update::ObservationWithValue(12, Instant::now());
/// gauge.update(&update);
///
/// assert_eq!(Some(12), gauge.get());
/// ```
pub struct Gauge {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<u64>,
}

impl Gauge {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            title: None,
            description: None,
            value: None,
        }
    }

    pub fn set(&mut self, v: u64) {
        self.value = Some(v);
    }

    pub fn get(&self) -> Option<u64> {
        self.value
    }

    /// Gets the name of this `Gauge`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `Gauge`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `Gauge`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `Gauge`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }
}

impl Instrument for Gauge {}

impl PutsSnapshot for Gauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        if let Some(v) = self.value {
            into.items.push((self.name.clone(), v.into()));
        }
    }
}

impl Updates for Gauge {
    fn update(&mut self, with: &Update) {
        match *with {
            Update::ObservationWithValue(v, _) => self.set(v),
            _ => (),
        }
    }
}

impl Descriptive for Gauge {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
