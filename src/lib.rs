use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Instant;

mod internal;
pub mod instruments;
pub mod snapshot;

pub enum Observation<T> {
    Observed(T, u64, Instant),
    ObservedOne(T, Instant),
    ObservedOneValue(T, u64, Instant),
}

impl<T> Observation<T> {
    pub fn key(&self) -> &T {
        match *self {
            Observation::Observed(ref k, _, _) => k,
            Observation::ObservedOne(ref k, _) => k,
            Observation::ObservedOneValue(ref k, _, _) => k,
        }
    }
}

pub trait SendsObservations<T> {
    /// Send an observation for recording
    fn send(&self, observation: Observation<T>);

    /// Observed `n` occurences at time `t`
    fn observed(&self, id: T, n: u64, t: Instant) {
        self.send(Observation::Observed(id, n, t))
    }

    /// Observed one occurence at time `t`
    fn observed_one(&self, id: T, t: Instant) {
        self.send(Observation::ObservedOne(id, t))
    }

    /// Observed one occurence with value `v` at time `t`
    fn observed_one_value(&self, id: T, v: u64, t: Instant) {
        self.send(Observation::ObservedOneValue(id, v, t))
    }

    /// Observed `n` occurences at now.
    fn observed_now(&self, id: T, n: u64) {
        self.observed(id, n, Instant::now())
    }

    /// Observed one occurence now
    fn observed_one_now(&self, id: T) {
        self.observed_one(id, Instant::now())
    }

    /// Observed one occurence with value `v`now
    fn observed_one_value_now(&self, id: T, v: u64) {
        self.observed_one_value(id, v, Instant::now())
    }
}

#[derive(Clone)]
pub struct ObservationsSender<T> {
    sender: mpsc::Sender<Observation<T>>,
}

impl<T> ObservationsSender<T>
where
    T: Display + Eq + Send + 'static,
{
    pub fn synced(&self) -> ObservationsSenderSync<T> {
        ObservationsSenderSync {
            sender: Arc::new(Mutex::new(self.sender.clone())),
        }
    }
}

impl<T> SendsObservations<T> for ObservationsSender<T> {
    fn send(&self, observation: Observation<T>) {
        if let Err(_err) = self.sender.send(observation) {
            // maybe log...
        }
    }
}

/// This is almost the same as the `ObservationSender`.
///
/// Since a `Sender` for a channel is not `Sync` this
/// struct wraps the `Sender` in an `Arc<Mutex<_>>` so that
/// it can be shared between threads.
#[derive(Clone)]
pub struct ObservationsSenderSync<T> {
    sender: Arc<Mutex<mpsc::Sender<Observation<T>>>>,
}

impl<T> ObservationsSenderSync<T>
where
    T: Display + Eq + Send + 'static,
{
}

impl<T> SendsObservations<T> for ObservationsSenderSync<T> {
    fn send(&self, observation: Observation<T>) {
        if let Err(_err) = self.sender.lock().unwrap().send(observation) {
            // maybe log...
        }
    }
}

pub struct MetrixReactor {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
