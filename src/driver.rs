use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use telemetry_receiver::{AcceptsSendableReceiver, ReceivesTelemetryData,
                         SendableReceivesTelemetryData};
use snapshot::TelemetrySnapshot;


pub struct TelemetryDriver {
    receivers: Arc<Mutex<Vec<(Option<String>, SendableReceivesTelemetryData)>>>,
    is_running: Arc<AtomicBool>,
}


impl TelemetryDriver {
    fn add_receiver_internal(&self, name: Option<String>, receiver: SendableReceivesTelemetryData) {
        self.receivers.lock().unwrap().push((name, receiver));
    }
}

impl ReceivesTelemetryData for TelemetryDriver {
    /// Receive and handle pending operations
    fn receive(&mut self, _max: u64) -> u64 {
        0
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

impl Default for TelemetryDriver {
    fn default() -> TelemetryDriver {
        let driver = TelemetryDriver {
            is_running: Arc::new(AtomicBool::new(true)),
            receivers: Arc::new(Mutex::new(Vec::new())),
        };

        start_telemetry_loop(driver.receivers.clone(), driver.is_running.clone());

        driver
    }
}

impl AcceptsSendableReceiver for TelemetryDriver {
    fn register_receiver(&self, receiver: SendableReceivesTelemetryData) {
        self.add_receiver_internal(None, receiver);
    }

    fn register_receiver_with_name<T: Into<String>>(
        &self,
        name: T,
        receiver: SendableReceivesTelemetryData,
    ) {
        self.add_receiver_internal(Some(name.into()), receiver);
    }
}

impl Drop for TelemetryDriver {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

fn start_telemetry_loop(
    receivers: Arc<Mutex<Vec<(Option<String>, SendableReceivesTelemetryData)>>>,
    is_running: Arc<AtomicBool>,
) {
    thread::spawn(move || telemetry_loop(&receivers, &is_running));
}

fn telemetry_loop(
    receivers: &Mutex<Vec<(Option<String>, SendableReceivesTelemetryData)>>,
    is_running: &AtomicBool,
) {
    loop {
        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        let started = Instant::now();
        do_a_run(receivers);
        let finished = Instant::now();
        let elapsed = finished - started;
        if elapsed < Duration::from_millis(5) {
            thread::sleep(Duration::from_millis(5) - elapsed)
        }
    }
}

fn do_a_run(receivers: &Mutex<Vec<(Option<String>, SendableReceivesTelemetryData)>>) {
    let mut receivers = receivers.lock().unwrap();

    for &mut (_, ref mut receiver) in receivers.iter_mut() {
        let _ = receiver.receive(1000);
    }
}
