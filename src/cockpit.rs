use Observation;
use instruments::*;
use snapshot::{ItemKind, Snapshot};
use util;

/// A cockpit groups panels.
///
/// Use a cockpit to group panels that are somehow related.
/// Since the Cockpit is generic over its label you can
/// use an enum as a label for grouping panels easily.
pub struct Cockpit<L> {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    panels: Vec<Panel<L>>,
    value_scaling: Option<ValueScaling>,
}

impl<L> Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    pub fn new<T: Into<String>>(name: T, value_scaling: Option<ValueScaling>) -> Cockpit<L> {
        let mut cockpit = Cockpit::default();
        cockpit.name = Some(name.into());
        cockpit.value_scaling = value_scaling;
        cockpit
    }

    pub fn without_name(value_scaling: Option<ValueScaling>) -> Cockpit<L> {
        let mut cockpit = Cockpit::default();
        cockpit.value_scaling = value_scaling;
        cockpit
    }

    pub fn with_value_scaling<T: Into<String>>(
        &mut self,
        name: T,
        value_scaling: ValueScaling,
    ) -> Cockpit<L> {
        Cockpit::new(name, Some(value_scaling))
    }
    pub fn without_value_scaling<T: Into<String>>(&mut self, name: T) -> Cockpit<L> {
        Cockpit::new(name, None)
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn set_value_scaling(&mut self, value_scaling: ValueScaling) {
        self.value_scaling = Some(value_scaling)
    }

    pub fn add_panel(&mut self, panel: Panel<L>) -> bool {
        if self.panels
            .iter()
            .find(|x| x.label == panel.label)
            .is_some()
        {
            false
        } else {
            self.panels.push(panel);
            true
        }
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        self.panels
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive))
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
            value_scaling: None,
        }
    }
}

impl<L> HandlesObservations for Cockpit<L>
where
    L: Clone + Eq + Send + 'static,
{
    type Label = L;

    fn handle_observation(&mut self, observation: &Observation<Self::Label>) {
        let LabelAndObservation(label, update) = observation.into();

        let update = if let Some(scaling) = self.value_scaling {
            update.scale(scaling)
        } else {
            update
        };

        self.panels
            .iter_mut()
            .filter(|p| p.label == label)
            .for_each(|p| p.update(&update));
    }

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

impl<L> ::Descriptive for Cockpit<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
