use std::time::{Duration, Instant};

use crate::instruments::{
    AcceptAllLabels, Instrument, InstrumentAdapter, LabelFilter, LabelPredicate, Update, Updates,
};
use crate::snapshot::{ItemKind, Snapshot};
use crate::util;
use crate::{Descriptive, PutsSnapshot};

/// A `DataDisplay` is 'tapped' by an `Observation`
/// and then stays on for some time.
///
///
/// The `DataDisplay` works exactly like a switch in
/// a staircase: You press the switch and the light will
/// stay on for some time. Here it is an `Observation` that
/// pushes the switch and the light is a boolean that will
/// be set to `true` for some time. When triggered while
/// already on, the time being `true` will be prolonged.
///
/// The state written to a `Snapshot` can be inverted.
pub struct DataDisplay {
    name: String,
    title: Option<String>,
    description: Option<String>,
    value: Option<ItemKind>,
    stay_on_for: Option<Duration>,
    stay_on_until: Option<Instant>,
}

impl DataDisplay {
    pub fn new<T: Into<String>>(name: T) -> DataDisplay {
        DataDisplay {
            name: name.into(),
            title: None,
            description: None,
            value: None,
            stay_on_for: None,
            stay_on_until: None,
        }
    }

    pub fn new_with_defaults<T: Into<String>>(name: T) -> DataDisplay {
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

    /// Sets duration after which the internal state switches back to `false`
    ///
    /// Default is 60 seconds
    pub fn set_stay_on_for(&mut self, d: Duration) {
        self.stay_on_for = Some(d);
    }

    /// Sets duration after which the internal state switches back to `false`
    ///
    /// Default is 60 seconds
    pub fn stay_on_for(mut self, d: Duration) -> Self {
        self.set_stay_on_for(d);
        self
    }

    /// Gets duration after which the internal state switches back to `false`
    pub fn get_stay_on_for(&self) -> Option<Duration> {
        self.stay_on_for
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
    pub fn value(&self) -> Option<ItemKind> {
        if let Some(stay_on_until) = self.stay_on_until {
            if stay_on_until >= Instant::now() {
                self.value.clone()
            } else {
                None
            }
        } else {
            self.value.clone()
        }
    }
}

impl Instrument for DataDisplay {}

impl PutsSnapshot for DataDisplay {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        if let Some(value) = self.value() {
            into.push(self.name.clone(), value);
        }
    }
}

impl Updates for DataDisplay {
    fn update(&mut self, update: &Update) -> usize {
        if let Some(value) = update.observed_value().and_then(|v| v.to_item_kind()) {
            if let Some(stay_on_for) = self.stay_on_for {
                self.stay_on_until = Some(Instant::now() + stay_on_for);
            }
            self.value = Some(value);
            1
        } else {
            0
        }
    }
}

impl Descriptive for DataDisplay {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
