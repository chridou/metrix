use std::time::Instant;
use std::fmt::Display;

use Observation;
use snapshot::*;

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

/// Something that can react on `Observation<L>`s.
///
/// You can use this to implement your own Metrics.
pub trait HandlesObservations {
    type Label;
    fn handle_observation(&mut self, observation: Observation<Self::Label>);
    fn name(&self) -> &str;
}

/// A cockpit groups panels.
///
/// Use a cockpit to group panels that are somehow related.
/// Since the Cockpit is generic over its label you can
/// use an enum as a label for grouping panels easily.
pub struct Cockpit<L> {
    name: String,
    panels: Vec<(L, Panel)>,
}

impl<L> Cockpit<L>
where
    L: Sized + Display + Clone + Eq,
{
    pub fn new<N: Into<String>>(name: N) -> Cockpit<L> {
        Cockpit {
            name: name.into(),
            panels: Vec::new(),
        }
    }

    pub fn add_panel(&mut self, label: L, panel: Panel) -> bool {
        if self.panels.iter().find(|x| x.0 == label).is_some() {
            false
        } else {
            self.panels.push((label, panel));
            true
        }
    }

    pub fn snapshot(&self) -> CockpitSnapshot {
        CockpitSnapshot {
            name: self.name.clone(),
            panels: self.panels
                .iter()
                .map(|&(ref l, ref p)| (l.to_string(), p.snapshot()))
                .collect(),
        }
    }
}

impl<L> HandlesObservations for Cockpit<L>
where
    L: Sized + Clone + Eq,
{
    type Label = L;

    fn handle_observation(&mut self, observation: Observation<Self::Label>) {
        if let Some(&mut (_, ref mut panel)) =
            self.panels.iter_mut().find(|p| &p.0 == observation.label())
        {
            panel.update(&observation.into())
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub trait Updates {
    fn update(&mut self, with: &Update);
}

/// The panel shows recorded
/// observations of the same label
/// in different flavours.
///
/// Let's say you want to monitor the successful requests
/// of a specific endpoint of your REST API.
/// You would then create a panel for this and might
/// want to add a counter and a meter and a histogram
/// to track latencies.
pub struct Panel {
    counter: Option<Counter>,
    gauge: Option<Gauge>,
    meter: Option<Meter>,
    histogram: Option<Histogram>,
}

impl Panel {
    pub fn add_counter(&mut self, counter: Counter) {
        self.counter = Some(counter);
    }

    pub fn add_gauge(&mut self, gauge: Gauge) {
        self.gauge = Some(gauge);
    }

    pub fn snapshot(&self) -> PanelSnapshot {
        PanelSnapshot {
            counter: self.counter.as_ref().map(|x| x.snapshot()),
            gauge: self.gauge.as_ref().map(|x| x.snapshot()),
            meter: self.meter.as_ref().map(|x| x.snapshot()),
            histogram: self.histogram.as_ref().map(|x| x.snapshot()),
        }
    }
}

impl Default for Panel {
    fn default() -> Panel {
        Panel {
            counter: None,
            gauge: None,
            meter: None,
            histogram: None,
        }
    }
}

impl Updates for Panel {
    fn update(&mut self, with: &Update) {
        self.counter.iter_mut().for_each(|x| x.update(with));
        self.gauge.iter_mut().for_each(|x| x.update(with));
        self.meter.iter_mut().for_each(|x| x.update(with));
        self.histogram.iter_mut().for_each(|x| x.update(with));
    }
}

pub struct Counter {
    name: String,
    count: u64,
}

impl Counter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Counter {
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

    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            name: self.name.clone(),
        }
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
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Gauge {
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

    pub fn snapshot(&self) -> GaugeSnapshot {
        GaugeSnapshot {
            name: self.name.clone(),
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

pub struct Meter {
    name: String,
}

impl Meter {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Meter {
        Meter { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn snapshot(&self) -> MeterSnapshot {
        MeterSnapshot {
            name: self.name.clone(),
        }
    }
}

impl Updates for Meter {
    fn update(&mut self, _with: &Update) {}
}

pub struct Histogram {
    name: String,
}

impl Histogram {
    pub fn new_with_defaults<T: Into<String>>(name: T) -> Histogram {
        Histogram { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn snapshot(&self) -> HistogramSnapshot {
        HistogramSnapshot {
            name: self.name.clone(),
        }
    }
}

impl Updates for Histogram {
    fn update(&mut self, _with: &Update) {}
}
