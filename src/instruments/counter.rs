use instruments::Update;
use instruments::Updates;

use snapshot::{ItemKind, Snapshot};
use Descriptive;
use util;

/// A simple ever increasing counter
pub struct Counter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    count: u64,
}

impl Counter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
            title: None,
            description: None,
            count: 0,
        }
    }

    pub fn inc(&mut self) {
        self.count += 1;
    }

    pub fn inc_by(&mut self, n: u64) {
        self.count += n;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = name.into();
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_prefixed_descriptives(self, &self.name, into, descriptive);
        into.items
            .push((self.name.clone(), ItemKind::UInt(self.count)));
    }
}

impl Updates for Counter {
    fn update(&mut self, with: &Update) {
        match *with {
            Update::Observation(_) => self.inc(),
            Update::Observations(n, _) => self.inc_by(n),
            Update::ObservationWithValue(_, _) => self.inc(),
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
