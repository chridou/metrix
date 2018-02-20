//! Transmitting observations and grouping metrics.
use std::sync::mpsc;
use std::time::Instant;

use {Observation, PutsSnapshot, TelemetryTransmitter};
use Descriptive;
use instruments::Panel;
use cockpit::{Cockpit, HandlesObservations};
use snapshot::{ItemKind, Snapshot};
use util;

/// Implementors can group everything that can process
/// `TelemetryMessage`s.
///
/// Since `PutsSnapshot` implementors can be added almost everywhere
/// the `add_snapshooter` method is placed here, too.
pub trait AggregatesProcessors {
    /// Add a processor.
    fn add_processor<P: ProcessesTelemetryMessages>(&mut self, processor: P);
    /// Add a snapshooter.
    fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S);
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

/// The result of processing
/// messages.
///
/// Used for making decisions for further processing
/// within the `TelemetryDriver`
pub struct ProcessingOutcome {
    pub processed: usize,
    pub dropped: usize,
}

impl ProcessingOutcome {
    /// Simply add the corresponding elements
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

/// Can process `TelemetryMessage`.
///
/// This is the counterpart of `TransmitsTelemetryData`.
///
/// Since this mostly results in metrics this
/// trait also requires the capability to write `Snapshot`s.
pub trait ProcessesTelemetryMessages: PutsSnapshot + Send + 'static {
    /// Receive and handle pending operations
    fn process(&mut self, max: usize, drop_deadline: Instant) -> ProcessingOutcome;
}

/// The counterpart of the `TelemetryTransmitter`. It receives the
/// `Observation`s and other messages and processes them.
///
/// A `TelemetryProcessor` is tied to a specific kind of label
/// which is used to determine which metrics are triggered.
///
/// The `TelemetryProcessor<L>` owns a `Receiver`
/// for `TelemetryMessage<L>`.
pub struct TelemetryProcessor<L> {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    cockpits: Vec<Cockpit<L>>,
    handlers: Vec<Box<HandlesObservations<Label = L>>>,
    receiver: mpsc::Receiver<TelemetryMessage<L>>,
    snapshooters: Vec<Box<PutsSnapshot>>,
}

impl<L> TelemetryProcessor<L>
where
    L: Clone + Eq + Send + 'static,
{
    /// Creates a `TelemetryTransmitter` and the corresponding `TelemetryProcessor`
    ///
    /// The `name` will cause a grouping in the `Snapshot`.
    pub fn new_pair<T: Into<String>>(name: T) -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            name: Some(name.into()),
            title: None,
            description: None,
            cockpits: Vec::new(),
            handlers: Vec::new(),
            snapshooters: Vec::new(),
            receiver: rx,
        };

        (transmitter, receiver)
    }

    /// Creates a `TelemetryTransmitter` and the corresponding `TelemetryProcessor`
    ///
    /// No grouping will occur unless the name is set.
    pub fn new_pair_without_name() -> (TelemetryTransmitter<L>, TelemetryProcessor<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryProcessor {
            name: None,
            title: None,
            description: None,
            cockpits: Vec::new(),
            handlers: Vec::new(),
            snapshooters: Vec::new(),
            receiver: rx,
        };

        (transmitter, receiver)
    }

    /// Add a `Cockpit`
    pub fn add_cockpit(&mut self, cockpit: Cockpit<L>) {
        self.cockpits.push(cockpit)
    }

    /// Returns all contained `Cockpit`s
    pub fn cockpits(&self) -> Vec<&Cockpit<L>> {
        self.cockpits.iter().map(|p| p).collect()
    }

    /// Add a (custom) handler for `Observation`s.
    pub fn add_handler(&mut self, handler: Box<HandlesObservations<Label = L>>) {
        self.handlers.push(handler);
    }

    /// Returns all the handlers
    pub fn handlers(&self) -> Vec<&HandlesObservations<Label = L>> {
        self.handlers.iter().map(|h| &**h).collect()
    }

    /// Add a snapshooter that simply creates some `Snapshot` defined
    /// by it's internal logic. Usually it polls something when a
    /// `Snapshot` is requested.
    pub fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S) {
        self.snapshooters.push(Box::new(snapshooter));
    }

    pub fn snapshooters(&self) -> Vec<&PutsSnapshot> {
        self.snapshooters.iter().map(|p| &**p).collect()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    /// Sets the name which will cause a grouoing in the `Snapshot`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);

        self.cockpits
            .iter()
            .for_each(|c| c.put_snapshot(into, descriptive));

        self.handlers
            .iter()
            .for_each(|h| h.put_snapshot(into, descriptive));

        self.snapshooters
            .iter()
            .for_each(|s| s.put_snapshot(into, descriptive));
    }
}

impl<L> ProcessesTelemetryMessages for TelemetryProcessor<L>
where
    L: Clone + Eq + Send + 'static,
{
    fn process(&mut self, max: usize, drop_deadline: Instant) -> ProcessingOutcome {
        let mut num_received = 0;
        let mut processed = 0;
        let mut dropped = 0;
        while num_received < max {
            match self.receiver.try_recv() {
                Ok(TelemetryMessage::Observation(obs)) => {
                    if obs.timestamp() <= drop_deadline {
                        dropped += 1;
                    } else {
                        self.cockpits
                            .iter_mut()
                            .for_each(|c| c.handle_observation(&obs));
                        self.handlers
                            .iter_mut()
                            .for_each(|h| h.handle_observation(&obs));
                        processed += 1;
                    }
                }
                Ok(TelemetryMessage::AddCockpit(c)) => {
                    self.add_cockpit(c);
                    processed += 1;
                }
                Ok(TelemetryMessage::AddHandler(h)) => {
                    self.add_handler(h);
                    processed += 1;
                }
                Ok(TelemetryMessage::AddPanel {
                    cockpit_name,
                    panel,
                }) => {
                    if let Some(ref mut cockpit) = self.cockpits
                        .iter_mut()
                        .find(|c| c.name() == Some(&cockpit_name))
                    {
                        let _ = cockpit.add_panel(panel);
                    }
                    processed += 1;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
            num_received += 1;
        }

        ProcessingOutcome { processed, dropped }
    }
}

impl<L> PutsSnapshot for TelemetryProcessor<L>
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

impl<L> Descriptive for TelemetryProcessor<L> {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

/// A building block for grouping
pub struct ProcessorMount {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    processors: Vec<Box<ProcessesTelemetryMessages>>,
    snapshooters: Vec<Box<PutsSnapshot>>,
}

impl ProcessorMount {
    /// Creates a new instance.
    ///
    /// Even though a name is optional having one
    /// is the default since this struct is mostly used to group
    /// other components.
    pub fn new<T: Into<String>>(name: T) -> ProcessorMount {
        let mut mount = ProcessorMount::default();
        mount.set_name(name);
        mount
    }

    /// Returns the name
    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    /// Sets the name of this `ProcessorMount`
    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    /// Returns the processors in this `ProcessorMount`
    pub fn processors(&self) -> Vec<&ProcessesTelemetryMessages> {
        self.processors.iter().map(|p| &**p).collect()
    }

    /// Returns the snapshooters of this `ProcessorMount`
    pub fn snapshooters(&self) -> Vec<&PutsSnapshot> {
        self.snapshooters.iter().map(|s| &**s).collect()
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        self.processors
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));

        self.snapshooters
            .iter()
            .for_each(|s| s.put_snapshot(into, descriptive));
    }
}

impl Default for ProcessorMount {
    fn default() -> ProcessorMount {
        ProcessorMount {
            name: None,
            title: None,
            description: None,
            processors: Vec::new(),
            snapshooters: Vec::new(),
        }
    }
}

impl AggregatesProcessors for ProcessorMount {
    fn add_processor<P: ProcessesTelemetryMessages>(&mut self, processor: P) {
        self.processors.push(Box::new(processor));
    }

    fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S) {
        self.snapshooters.push(Box::new(snapshooter));
    }
}

impl ProcessesTelemetryMessages for ProcessorMount {
    fn process(&mut self, max: usize, drop_deadline: Instant) -> ProcessingOutcome {
        let mut aggregated = ProcessingOutcome::default();

        for processor in self.processors.iter_mut() {
            aggregated.combine_with(&processor.process(max, drop_deadline));
        }

        aggregated
    }
}

impl PutsSnapshot for ProcessorMount {
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
