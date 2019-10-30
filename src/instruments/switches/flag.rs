use instruments::{Instrument, Update, Updates};
use snapshot::Snapshot;
use util;
use {Descriptive, PutsSnapshot};

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
    show_inverted: Option<(String, bool)>,
}

impl Flag {
    pub fn new_with_defaults<T: Into<String>>(name: T, initial_state: Option<bool>) -> Self {
        Self {
            name: name.into(),
            title: None,
            description: None,
            state: initial_state,
            show_inverted: None,
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

    /// Show the inverted value. Name will be prefixed with `prefix`.
    #[deprecated(since = "0.9.12", note = "use 'show_inverted_prefixed'")]
    pub fn show_inverted<T: Into<String>>(&mut self, prefix: T) {
        self.show_inverted_prefixed(prefix)
    }

    /// Show the inverted value. Name will be prefixed with `prefix`.
    pub fn show_inverted_prefixed<T: Into<String>>(&mut self, prefix: T) {
        self.show_inverted = Some((prefix.into(), true))
    }

    /// Show the inverted value. Name will be postfixed with `postfix`.
    pub fn show_inverted_postfix<T: Into<String>>(&mut self, postfix: T) {
        self.show_inverted = Some((postfix.into(), false));
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
            if let Some((inverted_tag, prefixed)) = &self.show_inverted {
                let label = if *prefixed {
                    format!("{}{}", inverted_tag, self.name)
                } else {
                    format!("{}{}", self.name, inverted_tag)
                };
                into.items.push((label, (!state).into()));
            }
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
