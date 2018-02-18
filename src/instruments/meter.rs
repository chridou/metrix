use instruments::Update;
use instruments::Updates;
use std::time::{Duration, Instant};

use metrics::metrics::{Meter as MMeter, StdMeter};

use Descriptive;
use snapshot::{ItemKind, MeterRate, MeterSnapshot, Snapshot};
use util;

/// For measuring rates, e.g. request/s
pub struct Meter {
    name: String,
    title: Option<String>,
    description: Option<String>,
    last_tick: Instant,
    inner_meter: StdMeter,
}

impl Meter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
        Meter {
            name: name.into(),
            title: None,
            description: None,
            last_tick: Instant::now(),
            inner_meter: StdMeter::default(),
        }
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
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level, descriptive);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if self.last_tick.elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick()
        }

        let snapshot = self.inner_meter.snapshot();

        let meter_snapshot = MeterSnapshot {
            one_minute: MeterRate {
                count: snapshot.count as u64,
                rate: snapshot.rates[0],
            },
            five_minutes: MeterRate {
                count: snapshot.count as u64,
                rate: snapshot.rates[1],
            },
            fifteen_minutes: MeterRate {
                count: snapshot.count as u64,
                rate: snapshot.rates[2],
            },
        };
        meter_snapshot.put_snapshot(into);
    }
}

impl Updates for Meter {
    fn update(&mut self, with: &Update) {
        if self.last_tick.elapsed() >= Duration::from_secs(5) {
            self.inner_meter.tick()
        }

        match *with {
            Update::ObservationWithValue(_, _) => self.inner_meter.mark(1),
            Update::Observations(n, _) => self.inner_meter.mark(n as i64),
            _ => (),
        }
    }
}

impl Descriptive for Meter {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
