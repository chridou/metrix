use std::time::{Duration, Instant};

use metrics::metrics::{Meter as MMeter, StdMeter};

use instruments::{Instrument, Update, Updates};

use {Descriptive, PutsSnapshot};
use snapshot::{ItemKind, Snapshot};
use util;

/// For measuring rates, e.g. request/s
///
/// This meter count occurences. An occurrence with values is
/// counted as 1 occurence.
///
/// To get rates on values use `instruments::other_instruments::ValeMeter`
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

    fn put_values_into_snapshot(&self, into: &mut Snapshot) {
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

impl Instrument for Meter {}

impl PutsSnapshot for Meter {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_postfixed_descriptives(self, &self.name, into, descriptive);
        let mut new_level = Snapshot::default();
        self.put_values_into_snapshot(&mut new_level);
        into.push(self.name.clone(), ItemKind::Snapshot(new_level));
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
            Update::Observation(_) => self.inner_meter.mark(1),
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

struct MeterSnapshot {
    pub one_minute: MeterRate,
    pub five_minutes: MeterRate,
    pub fifteen_minutes: MeterRate,
}

impl MeterSnapshot {
    pub fn put_snapshot(&self, into: &mut Snapshot) {
        let mut one_minute = Snapshot::default();
        self.one_minute.put_snapshot(&mut one_minute);
        into.items
            .push(("one_minute".to_string(), ItemKind::Snapshot(one_minute)));
        let mut five_minutes = Snapshot::default();
        self.five_minutes.put_snapshot(&mut five_minutes);
        into.items
            .push(("five_minutes".to_string(), ItemKind::Snapshot(five_minutes)));
        let mut fifteen_minutes = Snapshot::default();
        self.fifteen_minutes.put_snapshot(&mut fifteen_minutes);
        into.items.push((
            "fifteen_minutes".to_string(),
            ItemKind::Snapshot(fifteen_minutes),
        ));
    }
}

struct MeterRate {
    pub rate: f64,
    pub count: u64,
}

impl MeterRate {
    fn put_snapshot(&self, into: &mut Snapshot) {
        into.items.push(("rate".to_string(), self.rate.into()));
        into.items.push(("count".to_string(), self.count.into()));
    }
}
