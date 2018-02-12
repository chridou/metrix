use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::fmt::Display;

use {Observation, TelemetryTransmitter};
use instruments::{Cockpit, HandlesObservations};
use snapshot::TelemetrySnapshot;

pub type SendableReceivesTelemetryData = Box<ReceivesTelemetryData + Send + 'static>;

pub trait AcceptsSendableReceiver {
    fn register_receiver<T: Into<String>>(&self, name: T, receiver: SendableReceivesTelemetryData);
}

/// A message that can be handled by a `ReceivesTelemetryData`
pub enum TelemetryMessage<L> {
    /// An observation has been made
    Observation(Observation<L>),
    /// A `Cockpit` should be added
    AddCockpit(Cockpit<L>),
    /// An arbritrary `HandlesObservations` should be added
    AddHandler(Box<HandlesObservations<Label = L>>),
}

/// Can receive telemtry data also give snapshots
pub trait ReceivesTelemetryData {
    /// Receive and handle pending operations
    fn receive(&mut self, max: u64) -> u64;

    /// Get the snapshot.
    fn snapshot(&self) -> TelemetrySnapshot;
}

pub struct TelemetryReceiver<L> {
    cockpits: Vec<Cockpit<L>>,
    handlers: Vec<Box<HandlesObservations<Label = L>>>,
    receiver: mpsc::Receiver<TelemetryMessage<L>>,
}

impl<L> TelemetryReceiver<L> {
    pub fn new() -> (TelemetryTransmitter<L>, TelemetryReceiver<L>) {
        let (tx, rx) = mpsc::channel();

        let transmitter = TelemetryTransmitter { sender: tx };

        let receiver = TelemetryReceiver {
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver: rx,
        };

        (transmitter, receiver)
    }

    fn add_handler(&mut self, handler: Box<HandlesObservations<Label = L>>) {
        self.handlers.push(handler);
    }

    fn add_cockpit(&mut self, cockpit: Cockpit<L>) {
        self.cockpits.push(cockpit)
    }
}

impl<L> ReceivesTelemetryData for TelemetryReceiver<L>
where
    L: Clone + Display + Eq + Send + 'static,
{
    fn receive(&mut self, max: u64) -> u64 {
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
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
            n += 1;
        }
        n
    }

    fn snapshot(&self) -> TelemetrySnapshot {
        let mut collected = Vec::new();

        for c in &self.cockpits {
            collected.push(c.snapshot());
        }

        for h in &self.handlers {
            collected.push(h.snapshot());
        }

        TelemetrySnapshot::Cockpits(collected)
    }
}

pub struct GroupedReceivers {
    receivers: Arc<Mutex<Vec<(String, SendableReceivesTelemetryData)>>>,
}

impl AcceptsSendableReceiver for GroupedReceivers {
    fn register_receiver<T: Into<String>>(&self, name: T, receiver: SendableReceivesTelemetryData) {
        self.receivers.lock().unwrap().push((name.into(), receiver));
    }
}

impl ReceivesTelemetryData for GroupedReceivers {
    fn receive(&mut self, max: u64) -> u64 {
        let mut sum = 0;
        let mut receivers = self.receivers.lock().unwrap();

        for &mut (_, ref mut receiver) in receivers.iter_mut() {
            let n = receiver.receive(max);
            sum += n;
        }
        sum
    }

    fn snapshot(&self) -> TelemetrySnapshot {
        let mut collected = Vec::new();
        let receivers = self.receivers.lock().unwrap();

        for &(ref name, ref receiver) in receivers.iter() {
            let snapshot = receiver.snapshot();
            collected.push((name.clone(), snapshot));
        }

        TelemetrySnapshot::Group(collected)
    }
}

impl Default for GroupedReceivers {
    fn default() -> GroupedReceivers {
        GroupedReceivers {
            receivers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
