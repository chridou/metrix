//! Instruments that poll their values when a snapshot is requested.
//!
//! This is useful when you need to query values like currently opened DB
//! connections etc.

use snapshot::*;
use util;
use {Descriptive, PutsSnapshot};

/// Create an instrument that delivers metrics based on querying values
/// when a `Snapshot` is requested.
///
/// The `Snapshot` can be generated from anything that
/// implements `PutsSnapshot`.
pub struct PollingInstrument<P> {
    /// If `create_group_with_name` is true, this name will create a new named
    /// group.
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub poll: P,
    pub create_group_with_name: bool,
}

impl<P> PollingInstrument<P>
where
    P: PutsSnapshot,
{
    pub fn new_with_defaults<T: Into<String>>(name: T, poll: P) -> PollingInstrument<P> {
        PollingInstrument {
            name: name.into(),
            title: None,
            description: None,
            poll,
            create_group_with_name: false,
        }
    }

    /// Gets the name of this `PollingInstrument`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the name if this `PollingInstrument`.
    ///
    /// The name is a path segment within a `Snapshot` if
    /// `self.create_group_with_name` is set to true.
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    /// Returns whether a new group with the instruments name
    /// shall be craeted in the snapshot.
    pub fn create_group_with_name(&self) -> bool {
        self.create_group_with_name
    }

    /// Set to `true` if you want to create a new group
    /// with the name of this instrument when a `Snapshot`
    /// is requested.
    pub fn set_create_group_with_name(&mut self, create_group: bool) {
        self.create_group_with_name = create_group;
    }

    /// Sets the `title` of this `PollingInstrument`.
    ///
    /// A title can be part of a descriptive `Snapshot`
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the `description` of this `PollingInstrument`.
    ///
    /// A description can be part of a descriptive `Snapshot`
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        self.poll.put_snapshot(into, descriptive);
    }
}

impl<P> Descriptive for PollingInstrument<P> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

impl<P> PutsSnapshot for PollingInstrument<P>
where
    P: PutsSnapshot,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        if self.create_group_with_name {
            let mut new_level = Snapshot::default();
            self.put_values_into_snapshot(&mut new_level, descriptive);
            into.items
                .push((self.name.clone(), ItemKind::Snapshot(new_level)));
        } else {
            self.put_values_into_snapshot(into, descriptive);
        }
    }
}
