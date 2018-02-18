use instruments::{Update, Updates};

use snapshot::{ItemKind, Snapshot};
use {Descriptive, PutsSnapshot};
use util;

/// Simply returns the value that has been observed last.
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn set(&mut self, v: u64) {
        self.value = Some(v);
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }
}

impl PutsSnapshot for Gauge {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_prefixed_descriptives(self, &self.name, into, descriptive);
        if let Some(v) = self.value {
            into.items.push((self.name.clone(), ItemKind::UInt(v)));
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
