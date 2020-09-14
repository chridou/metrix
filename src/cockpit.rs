//! Cockpits are used to monitor different aspects of a component
use std::time::{Duration, Instant};

use crate::instruments::*;
use crate::snapshot::{ItemKind, Snapshot};
use crate::util;
use crate::{HandlesObservations, Observation, PutsSnapshot};

/// A cockpit groups panels.
///
/// Use a cockpit to group panels that are somehow related.
/// Since the Cockpit is generic over its label you can
/// use an enum as a label for grouping panels easily.
///
pub struct Cockpit<L> {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    panels: Vec<Panel<L>>,
    handlers: Vec<Box<dyn HandlesObservations<Label = L>>>,
    snapshooters: Vec<Box<dyn PutsSnapshot>>,
    last_activity_at: Instant,
    max_inactivity_duration: Option<Duration>,
    show_activity_state: bool,
}

impl<L> Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    /// Creates a new instance.
    ///
    /// Even though the name is optional it is suggested to use
    /// a name for a cockpit since in most cases a `Cockpit` is
    /// a meaningful grouping for panels and instruments.
    pub fn new<T: Into<String>>(name: T) -> Cockpit<L> {
        let mut cockpit = Cockpit::default();
        cockpit.name = Some(name.into());
        cockpit
    }

    /// Creates a new `Cockpit` without a name.
    ///
    /// This will have the effect that there will be no grouping in the
    /// snapshot around the contained components.
    pub fn without_name() -> Cockpit<L> {
        Cockpit::default()
    }

    /*
    /// Returns the name of this cockpit.
    ///
    /// If there is a name set, this will group the inner components in the
    /// snapshot.
    #[deprecated(
        since = "0.10.6",
        note = "use get_name. this method will change its signature"
    )]
    pub fn name(&self) -> Option<&str> {
        self.get_name()
    }*/

    /// Returns the name of this cockpit.
    ///
    /// If there is a name set, this will group the inner components in the
    /// snapshot.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets a name for this `Cockpit`. This will also enable grouping.
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    /// Sets the title which will be displayed if a descriptive snapshot is
    /// requested.
    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    /// Sets the description which will be displayed if a descriptive snapshot
    /// is requested.
    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    /// Sets the maximum amount of time this cockpit may be
    /// inactive until no more snapshots are taken
    pub fn set_inactivity_limit(&mut self, limit: Duration) {
        self.max_inactivity_duration = Some(limit);
    }

    /// Set whether to show if the cockpit is inactive or not
    /// if `inactivity_limit` is set.
    ///
    /// The default is `true`. Only has an effect if a `inactivity_limit`
    /// is set.
    pub fn set_show_activity_state(&mut self, show: bool) {
        self.show_activity_state = show;
    }

    /// Set whether to show if the cockpit is inactive or not
    /// if `inactivity_limit` is set.
    ///
    /// The default is `true`. Only has an effect if a `inactivity_limit`
    /// is set.
    pub fn show_activity_state(mut self, show: bool) -> Self {
        self.set_show_activity_state(show);
        self
    }

    /// Add a `Panel` to this cockpit.
    ///
    /// A `Panel` will receive only those `Observation`s where
    /// labels match.
    ///
    /// There can **not** be multiple `Panel`s for the same label.
    /// If one already exists, the new one will not be added.
    pub fn add_panel(&mut self, panel: Panel<L>) {
        if let Some(name) = panel.name() {
            if self
                .panels
                .iter()
                .any(|p| p.name().map(|n| n == name).unwrap_or(false))
            {
                return;
            }
        }
        self.panels.push(panel)
    }

    /// Removes a `Panel` from this cockpit.
    ///
    /// Removes the `Panel` with the given name from this cockpit a
    /// a `Panel` with the given name exists
    pub fn remove_panel<T: AsRef<str>>(&mut self, name: T) {
        self.panels
            .retain(|p| p.name().map(|n| n != name.as_ref()).unwrap_or(true))
    }

    /// Add a `Panel` to this cockpit.
    ///
    /// A `Panel` will receive only those `Observation`s where
    /// labels match.
    ///
    /// There can be multiple `Panel`s for the same label.
    pub fn panel(mut self, panel: Panel<L>) -> Self {
        self.add_panel(panel);
        self
    }

    /// Returns the `Panel`s
    pub fn get_panels(&self) -> Vec<&Panel<L>> {
        self.panels.iter().collect()
    }

    /// Returns the `Panel`s mutable
    pub fn get_panels_mut(&mut self) -> Vec<&mut Panel<L>> {
        self.panels.iter_mut().collect()
    }

    /// Add a handler. This can be custom logic for
    /// metrics.
    ///
    /// A handler will be passed all `Observation`s unlike
    /// a `Panel` which will only receive `Observation`s where
    /// the label matches.
    pub fn add_handler<T>(&mut self, handler: T)
    where
        T: HandlesObservations<Label = L>,
    {
        self.handlers.push(Box::new(handler))
    }

    /*
    /// Returns all the handlers.
    #[deprecated(
        since = "0.10.6",
        note = "use get_handlers. this method will change its signature"
    )]
    pub fn handlers(&self) -> Vec<&dyn HandlesObservations<Label = L>> {
        self.get_handlers()
    }*/

    /// Returns all the handlers.
    pub fn get_handlers(&self) -> Vec<&dyn HandlesObservations<Label = L>> {
        self.handlers.iter().map(|h| &**h).collect()
    }

    pub fn handler<H: HandlesObservations<Label = L>>(mut self, handler: H) -> Self {
        self.add_handler(handler);
        self
    }

    /// Adds a snapshooter.
    ///
    /// A snapshooter will only be invoked when a `Snapshot` is requested.
    /// It will never receive an `Observation`.
    pub fn add_snapshooter<T: PutsSnapshot>(&mut self, snapshooter: T) {
        self.snapshooters.push(Box::new(snapshooter));
    }

    /// Adds a snapshooter.
    ///
    /// A snapshooter will only be invoked when a `Snapshot` is requested.
    /// It will never receive an `Observation`.
    pub fn snapshooter<T: PutsSnapshot>(mut self, snapshooter: T) -> Self {
        self.add_snapshooter(snapshooter);
        self
    }

    /*
    /// Returns all snapshooters.
    #[deprecated(
        since = "0.10.6",
        note = "use get_snapshooters. this method will change its signature"
    )]
    pub fn snapshooters(&self) -> Vec<&dyn PutsSnapshot> {
        self.get_snapshooters()
    }*/

    /// Returns all snapshooters.
    pub fn get_snapshooters(&self) -> Vec<&dyn PutsSnapshot> {
        self.snapshooters.iter().map(|p| &**p).collect()
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);

        if let Some(d) = self.max_inactivity_duration {
            if self.show_activity_state {
                if self.last_activity_at.elapsed() > d {
                    into.items
                        .push(("_inactive".to_string(), ItemKind::Boolean(true)));
                    into.items
                        .push(("_active".to_string(), ItemKind::Boolean(false)));
                    return;
                } else {
                    into.items
                        .push(("_inactive".to_string(), ItemKind::Boolean(false)));
                    into.items
                        .push(("_active".to_string(), ItemKind::Boolean(true)));
                }
            }
        };

        self.panels
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));

        self.handlers
            .iter()
            .for_each(|h| h.put_snapshot(into, descriptive));

        self.snapshooters
            .iter()
            .for_each(|s| s.put_snapshot(into, descriptive));
    }
}

impl<L> PutsSnapshot for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if let Some(ref name) = self.name {
            let mut new_level = Snapshot::default();
            self.put_values_into_snapshot(&mut new_level, descriptive);
            into.items
                .push((name.clone(), ItemKind::Snapshot(new_level)));
        } else {
            self.put_values_into_snapshot(into, descriptive);
        }
    }
}

impl<L> Default for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn default() -> Cockpit<L> {
        Cockpit {
            name: None,
            title: None,
            description: None,
            panels: Vec::new(),
            handlers: Vec::new(),
            snapshooters: Vec::new(),
            last_activity_at: Instant::now(),
            max_inactivity_duration: None,
            show_activity_state: true,
        }
    }
}

impl<L> HandlesObservations for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) -> usize {
        self.last_activity_at = Instant::now();

        let mut instruments_updated = 0;

        self.handlers
            .iter_mut()
            .for_each(|h| instruments_updated += h.handle_observation(&observation));

        self.panels
            .iter_mut()
            .for_each(|p| instruments_updated += p.handle_observation(&observation));

        instruments_updated
    }
}

impl<L> crate::Descriptive for Cockpit<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
