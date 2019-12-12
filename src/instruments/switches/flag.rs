use crate::instruments::{Instrument, InstrumentAdapter, Update, Updates};
use crate::snapshot::Snapshot;
use crate::util;
use crate::{Descriptive, ObservedValue, PutsSnapshot};

use super::NameAlternation;

/// A `Flag` which can have the states `true` or `false`
///
/// The `Flag` reacts on observations with values. A value
/// of `0` sets the `Flag` to `false`, '1' will set the
/// `Flag` to `true`. For all other values the behaviour is undefined.
pub struct Flag {
    name: String,
    title: Option<String>,
    description: Option<String>,
    state: Option<bool>,
    show_inverted: Option<NameAlternation>,
}

impl Flag {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            state: None,
            show_inverted: None,
        }
    }

    pub fn new_with_state<T: Into<String>>(name: T, initial_state: bool) -> Self {
        Self::new(name).state(initial_state)
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

    pub fn state(mut self, initial_state: bool) -> Self {
        self.state = Some(initial_state);
        self
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

    /// Creates an `InstrumentAdapter` that makes this instrument react on
    /// observations with labels specified by the predicate.
    pub fn for_labels_by_predicate<L, P>(self, label_predicate: P) -> InstrumentAdapter<L, Self>
    where
        L: Eq,
        P: Fn(&L) -> bool + Send + 'static,
    {
        InstrumentAdapter::by_predicate(label_predicate, self)
    }

    /// Creates an `InstrumentAdapter` that makes this instrument to no
    /// observations.
    pub fn adapter<L: Eq>(self) -> InstrumentAdapter<L, Self> {
        InstrumentAdapter::deaf(self)
    }

    /// Returns the current state
    pub fn get_state(&self) -> Option<bool> {
        self.state
    }
}

impl Instrument for Flag {}

impl PutsSnapshot for Flag {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        if let Some(state) = self.state {
            into.items.push((self.name.clone(), state.into()));
            if let Some(alternation) = &self.show_inverted {
                let label = alternation.adjust_name(&self.name);
                into.items.push((label.into(), (!state).into()));
            }
        }
    }
}

impl Updates for Flag {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(ObservedValue::Bool(v), _) => {
                self.state = Some(v);
                1
            }
            Update::ObservationWithValue(ObservedValue::SignedInteger(v), _) => {
                self.state = Some(v != 0);
                1
            }
            Update::ObservationWithValue(ObservedValue::UnsignedInteger(v), _) => {
                self.state = Some(v != 0);
                1
            }
            _ => 0,
        }
    }
}

impl Descriptive for Flag {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
