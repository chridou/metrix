use instruments::{Instrument, Update, Updates};
use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot};

/// A `Flag` which can have the states `true` or `false`
///
/// The `Flag` reacts on observations with values. A value
/// of `0` sets the `Flag` to `false`, all other values set the
/// `Flag` to `true`.
pub struct Flag {
    name: String,
    title: Option<String>,
    description: Option<String>,
    state: Option<bool>,
}

impl Flag {
    pub fn new_with_defaults<T: Into<String>>(name: T, initial_state: Option<bool>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            state: initial_state,
        }
    }

    /// Gets the name of this `Flag`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `Flag`.
    ///
    /// The name is a path segment within a `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Sets the `title` of this `Flag`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `Flag`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    /// Returns the current state
    pub fn state(&self) -> Option<bool> {
        self.state
    }
}

impl Instrument for Flag {}

impl PutsSnapshot for Flag {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);

        if let Some(state) = self.state {
            into.items.push((self.name.clone(), state.into()));
        }
    }
}

impl Updates for Flag {
    fn update(&mut self, with: &Update) -> usize {
        match *with {
            Update::ObservationWithValue(value, _) => {
                if value == 0 {
                    self.state = Some(false)
                } else {
                    self.state = Some(true)
                }
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
