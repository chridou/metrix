use std::sync::mpsc;
use std::fmt::Display;

use {Observation, TelemetryTransmitter};
use instruments::{Cockpit, HandlesObservations, Panel};
use snapshot::MetricsSnapshot;

pub trait AggregatesProcessors {
    fn add_processor(&mut self, processor: Box<ProcessesTelemetryMessages>);
}

/// A message that can be handled by a `ReceivesTelemetryData`
pub(crate) enum TelemetryMessage<L> {
    /// An observation has been made
    Observation(Observation<L>),
    /// A `Cockpit` should be added
    AddCockpit(Cockpit<L>),
    /// An arbritrary `HandlesObservations` should be added
    AddHandler(Box<HandlesObservations<Label = L>>),
    /// Adds a panel to a cockpit with the given name
    ///
    /// This means the cockpit must have a name set.
    AddPanel {
        cockpit_name: String,
        label: L,
        panel: Panel,
    },
}

/// Can receive telemtry data also give snapshots
pub trait ProcessesTelemetryMessages: Send + 'static {
    /// Receive and handle pending operations
    fn process(&mut self, max: u64) -> u64;

    /// Get the snapshot.
    fn snapshot(&self) -> MetricsSnapshot;

    fn name(&self) -> Option<&str>;
}

pub struct TelemetryProcessor<L> {
    cockpits: Vec<Cockpit<L>>,
    handlers: Vec<Box<HandlesObservations<Label = L>>>,
    receiver: mpsc::Receiver<TelemetryMessage<L>>,
    name: Option<String>,
}

impl<L> TelemetryProcessor<L>
where
    L: Clone + Display + Eq + Send + 'static,
{
    pub fn new_pair<T: Into<String>>(name: T) -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver: rx,
            name: Some(name.into()),
        };

        (transmitter, receiver)
    }

    pub fn new_pair_without_name() -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver: rx,
            name: None,
        };

        (transmitter, receiver)
    }

    pub fn add_handler(&mut self, handler: Box<HandlesObservations<Label = L>>) {
        self.handlers.push(handler);
    }

    pub fn add_cockpit(&mut self, cockpit: Cockpit<L>) {
        self.cockpits.push(cockpit)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }
}

impl<L> ProcessesTelemetryMessages for TelemetryProcessor<L>
where
    L: Clone + Display + Eq + Send + 'static,
{
    fn process(&mut self, max: u64) -> u64 {
        let mut n = 0;
        while n < max {
            match self.receiver.try_recv() {
                Ok(TelemetryMessage::Observation(obs)) => {
                    self.cockpits
                        .iter_mut()
                        .for_each(|c| c.handle_observation(&obs));
                    self.handlers
                        .iter_mut()
                        .for_each(|h| h.handle_observation(&obs));
                }
                Ok(TelemetryMessage::AddCockpit(c)) => self.add_cockpit(c),
                Ok(TelemetryMessage::AddHandler(h)) => self.add_handler(h),
                Ok(TelemetryMessage::AddPanel {
                    cockpit_name,
                    label,
                    panel,
                }) => if let Some(ref mut cockpit) = self.cockpits
                    .iter_mut()
                    .find(|c| c.name() == Some(&cockpit_name))
                {
                    let _ = cockpit.add_panel(label, panel);
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
            n += 1;
        }
        n
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let mut collected = Vec::with_capacity(self.cockpits.len() + self.handlers.len());

        for c in &self.cockpits {
            collected.push(c.snapshot());
        }

        for h in &self.handlers {
            collected.push(h.snapshot());
        }

        if let Some(ref name) = self.name {
            MetricsSnapshot::Group(name.clone(), collected)
        } else {
            MetricsSnapshot::GroupWithoutName(collected)
        }
    }

    fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }
}

/// Use to build your hierarchy
pub struct ProcessorMount {
    name: Option<String>,
    processors: Vec<Box<ProcessesTelemetryMessages>>,
}

impl ProcessorMount {
    pub fn new<T: Into<String>>(name: T) -> ProcessorMount {
        ProcessorMount {
            name: Some(name.into()),
            processors: Vec::new(),
        }
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }
}

impl Default for ProcessorMount {
    fn default() -> ProcessorMount {
        ProcessorMount {
            name: None,
            processors: Vec::new(),
        }
    }
}

impl AggregatesProcessors for ProcessorMount {
    fn add_processor(&mut self, processor: Box<ProcessesTelemetryMessages>) {
        self.processors.push(processor);
    }
}

impl ProcessesTelemetryMessages for ProcessorMount {
    fn process(&mut self, max: u64) -> u64 {
        let mut sum = 0;

        for processor in self.processors.iter_mut() {
            let n = processor.process(max);
            sum += n;
        }
        sum
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let mut collected = Vec::with_capacity(self.processors.len());

        for processor in &self.processors {
            let snapshot = processor.snapshot();
            collected.push(snapshot);
        }

        if let Some(ref name) = self.name {
            MetricsSnapshot::Group(name.clone(), collected)
        } else {
            MetricsSnapshot::GroupWithoutName(collected)
        }
    }

    fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }
}
