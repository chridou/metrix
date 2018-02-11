use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Instant;

mod internal;
pub mod instruments;
pub mod snapshot;
pub mod telemetry_receiver;

#[derive(Clone)]
pub enum Observation<L> {
    Observed(L, u64, Instant),
    ObservedOne(L, Instant),
    ObservedOneValue(L, u64, Instant),
}

impl<L> Observation<L> where {
    pub fn label(&self) -> &L {
        match *self {
            Observation::Observed(ref l, _, _) => l,
            Observation::ObservedOne(ref l, _) => l,
            Observation::ObservedOneValue(ref l, _, _) => l,
        }
    }
}

pub trait TransmitsTelemetryData<L> {
    /// Collect an observation.
    fn transmit(&self, observation: Observation<L>);

    /// Observed `n` occurences at time `t`
    ///
    /// Convinience method. Simply calls `collect`
    fn observed(&self, label: L, n: u64, t: Instant) {
        self.transmit(Observation::Observed(label, n, t))
    }

    /// Observed one occurence at time `t`
    ///
    /// Convinience method. Simply calls `collect`
    fn observed_one(&self, label: L, t: Instant) {
        self.transmit(Observation::ObservedOne(label, t))
    }

    /// Observed one occurence with value `v` at time `t`
    ///
    /// Convinience method. Simply calls `collect`
    fn observed_one_value(&self, label: L, v: u64, t: Instant) {
        self.transmit(Observation::ObservedOneValue(label, v, t))
    }

    /// Observed `n` occurences at now.
    ///
    /// Convinience method. Simply calls `collect`
    fn observed_now(&self, label: L, n: u64) {
        self.observed(label, n, Instant::now())
    }

    /// Observed one occurence now
    ///
    /// Convinience method. Simply calls `collect`
    fn observed_one_now(&self, label: L) {
        self.observed_one(label, Instant::now())
    }

    /// Observed one occurence with value `v`now
    ///
    /// Convinience method. Simply calls `collect`
    fn observed_one_value_now(&self, label: L, v: u64) {
        self.observed_one_value(label, v, Instant::now())
    }
}

#[derive(Clone)]
pub struct TelemetryTransmitter<L> {
    sender: mpsc::Sender<Observation<L>>,
}

impl<L> TelemetryTransmitter<L>
where
    L: Send + 'static,
{
    pub fn synced(&self) -> TelemetryTransmitterSync<L> {
        TelemetryTransmitterSync {
            sender: Arc::new(Mutex::new(self.sender.clone())),
        }
    }
}

impl<L> TransmitsTelemetryData<L> for TelemetryTransmitter<L> {
    fn transmit(&self, observation: Observation<L>) {
        if let Err(_err) = self.sender.send(observation) {
            // maybe log...
        }
    }
}

/// This is almost the same as the `TelemetryTransmitter`.
///
/// Since a `Sender` for a channel is not `Sync` this
/// struct wraps the `Sender` in an `Arc<Mutex<_>>` so that
/// it can be shared between threads.
#[derive(Clone)]
pub struct TelemetryTransmitterSync<L> {
    sender: Arc<Mutex<mpsc::Sender<Observation<L>>>>,
}

impl<L> TelemetryTransmitterSync<L>
where
    L: Send + 'static,
{
}

impl<L> TransmitsTelemetryData<L> for TelemetryTransmitterSync<L> {
    fn transmit(&self, observation: Observation<L>) {
        if let Err(_err) = self.sender.lock().unwrap().send(observation) {
            // maybe log...
        }
    }
}

pub struct MetrixReactor {}
