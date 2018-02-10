use std::time::Instant;
use std::fmt::Display;

use Observation;

pub enum Update {
    Observations(u64, Instant),
    Observation(Instant),
    ObservationWithValue(u64, Instant),
}

impl<T> From<Observation<T>> for Update {
    fn from(obs: Observation<T>) -> Update {
        match obs {
            Observation::Observed(_, n, t) => Update::Observations(n, t),
            Observation::ObservedOne(_, t) => Update::Observation(t),
            Observation::ObservedOneValue(_, v, t) => Update::ObservationWithValue(v, t),
        }
    }
}

pub struct Cockpit<T> {
    name: String,
    panels: Vec<(T, Panel)>,
}

impl<T> Cockpit<T>
where
    T: Display + Eq + Send + 'static,
{
    pub fn new<N: Into<String>>(name: N) -> Cockpit<T> {
        Cockpit {
            name: name.into(),
            panels: Vec::new(),
        }
    }

    pub fn update(&mut self, observation: Observation<T>) {
        if let Some(&mut (_, ref mut panel)) =
            self.panels.iter_mut().find(|p| &p.0 == observation.key())
        {
            panel.update(&observation.into())
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub trait Updates {
    fn update(&mut self, with: &Update);
}

pub struct Panel {
    counter: Option<Counter>,
    gauge: Option<Gauge>,
}

impl Panel {
    pub fn add_counter(&mut self, counter: Counter) {
        self.counter = Some(counter);
    }

    pub fn add_gauge(&mut self, gauge: Gauge) {
        self.gauge = Some(gauge);
    }
}

impl Default for Panel {
    fn default() -> Panel {
        Panel {
            counter: None,
            gauge: None,
        }
    }
}

impl Updates for Panel {
    fn update(&mut self, with: &Update) {
        self.counter.iter_mut().for_each(|x| x.update(with));
        self.gauge.iter_mut().for_each(|x| x.update(with));
    }
}

pub struct Counter {
    name: String,
    count: u64,
}

impl Counter {
    pub fn new<T: Into<String>>(name: T) -> Counter {
        Counter {
            name: name.into(),
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

pub struct Gauge {
    name: String,
    value: u64,
}

impl Gauge {
    pub fn new<T: Into<String>>(name: T) -> Gauge {
        Gauge {
            name: name.into(),
            value: 0,
        }
    }

    pub fn set(&mut self, v: u64) {
        self.value = v;
    }

    pub fn name(&self) -> &str {
        &self.name
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
