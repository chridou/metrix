use std::sync::mpsc;

use Observation;
use instruments::{Cockpit, HandlesObservations};

pub enum TelemetryMessage<L> {
    Observation(Observation<L>),
    AddCockpit(Cockpit<L>),
    AddHandler(Box<HandlesObservations<Label = L>>),
}

/// A trait for receiving telemetry data
///
/// Even though the `Label` is intended to
/// label `Panel`s in a cockpit and a cockpit
/// should be exhaustive over `Label` you
/// can add multiple `Cockpit`s and `HandlesObservations`
/// to accomplish custom behaviour.
pub trait ReceivesTelemetryData {
    type Label: Sized;
    fn add_handler(&mut self, handler: Box<HandlesObservations<Label = Self::Label>>);
    fn add_cockpit(&mut self, cockpit: Cockpit<Self::Label>);
    fn name(&self) -> &str;

    fn receive(&mut self, max: u64) -> u64;
}

pub struct TelemetryReceiver<L> {
    name: String,
    cockpits: Vec<Cockpit<L>>,
    handlers: Vec<Box<HandlesObservations<Label = L>>>,
    receiver: mpsc::Receiver<TelemetryMessage<L>>,
}

impl<L> TelemetryReceiver<L> {
    pub fn new<N: Into<String>>(
        name: N,
        receiver: mpsc::Receiver<TelemetryMessage<L>>,
    ) -> TelemetryReceiver<L> {
        TelemetryReceiver {
            name: name.into(),
            cockpits: Vec::new(),
            handlers: Vec::new(),
            receiver,
        }
    }
}

impl<L> ReceivesTelemetryData for TelemetryReceiver<L>
where
    L: Clone + Eq,
{
    type Label = L;

    fn add_handler(&mut self, handler: Box<HandlesObservations<Label = L>>) {
        self.handlers.push(handler);
    }

    fn add_cockpit(&mut self, cockpit: Cockpit<Self::Label>) {
        self.cockpits.push(cockpit)
    }

    fn name(&self) -> &str {
        &self.name
    }

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
                Ok(TelemetryMessage::AddCockpit(c)) => self.cockpits.push(c),
                Ok(TelemetryMessage::AddHandler(h)) => self.handlers.push(h),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
            n += 1;
        }
        n
    }
}

pub struct TelemetryRunner {}
