use std::sync::mpsc;

use {Observation, TelemetryTransmitter};
use instruments::{Cockpit, Descriptive, HandlesObservations, Panel};
use snapshot::{ItemKind, Snapshot};
use util;

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
        panel: Panel<L>,
    },
}

pub struct ProcessingOutcome {
    pub processed: usize,
    pub dropped: usize,
}

impl ProcessingOutcome {
    pub fn combine_with(&mut self, other: &ProcessingOutcome) {
        self.processed += other.processed;
        self.dropped += other.dropped;
    }
}

impl Default for ProcessingOutcome {
    fn default() -> ProcessingOutcome {
        ProcessingOutcome {
            processed: 0,
            dropped: 0,
        }
    }
}

/// Can receive telemtry data also give snapshots
pub trait ProcessesTelemetryMessages: Send + 'static {
    /// Receive and handle pending operations
    fn process(&mut self, max: usize) -> ProcessingOutcome;

    /// Put the snapshot.
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool);
}

pub struct TelemetryProcessor<L> {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    cockpits: Vec<Cockpit<L>>,
    handlers: Vec<Box<HandlesObservations<Label = L>>>,
    receiver: mpsc::Receiver<TelemetryMessage<L>>,
}

impl<L> TelemetryProcessor<L>
where
    L: Clone + Eq + Send + 'static,
{
    pub fn new_pair<T: Into<String>>(name: T) -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            name: Some(name.into()),
            title: None,
            description: None,
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver: rx,
        };

        (transmitter, receiver)
    }

    pub fn new_pair_without_name() -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            name: None,
            title: None,
            description: None,
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver: rx,
        };

        (transmitter, receiver)
    }

    pub fn add_handler(&mut self, handler: Box<HandlesObservations<Label = L>>) {
        self.handlers.push(handler);
    }

    pub fn add_cockpit(&mut self, cockpit: Cockpit<L>) {
        self.cockpits.push(cockpit)
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        for c in &self.cockpits {
            c.put_snapshot(into, descriptive);
        }

        for h in &self.handlers {
            h.put_snapshot(into, descriptive);
        }
    }
}

impl<L> ProcessesTelemetryMessages for TelemetryProcessor<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn process(&mut self, max: usize) -> ProcessingOutcome {
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
                    panel,
                }) => if let Some(ref mut cockpit) = self.cockpits
                    .iter_mut()
                    .find(|c| c.name() == Some(&cockpit_name))
                {
                    let _ = cockpit.add_panel(panel);
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
            n += 1;
        }

        ProcessingOutcome {
            processed: n,
            dropped: 0,
        }
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

impl<L> Descriptive for TelemetryProcessor<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

/// Use to build your hierarchy
pub struct ProcessorMount {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    processors: Vec<Box<ProcessesTelemetryMessages>>,
}

impl ProcessorMount {
    pub fn new<T: Into<String>>(name: T) -> ProcessorMount {
        let mut mount = ProcessorMount::default();
        mount.set_name(name);
        mount
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        for p in &self.processors {
            p.put_snapshot(into, descriptive);
        }
    }
}

impl Default for ProcessorMount {
    fn default() -> ProcessorMount {
        ProcessorMount {
            name: None,
            title: None,
            description: None,
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
    fn process(&mut self, max: usize) -> ProcessingOutcome {
        let mut aggregated = ProcessingOutcome::default();

        for processor in self.processors.iter_mut() {
            aggregated.combine_with(&processor.process(max));
        }

        aggregated
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

impl Descriptive for ProcessorMount {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}
