use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use processor::{AggregatesProcessors, ProcessesTelemetryMessages};
use snapshot::MetricsSnapshot;

/// Triggers registered `ProcessesTelemetryMessages` to
/// poll for messages.
///
/// Runs its own background thread. The thread stops once
/// this struct is dropped.
///
/// A `TelemetryDriver` can be 'mounted' into the hierarchy.
/// If done so, it will still poll its children on its own thread
/// independently.
pub struct TelemetryDriver {
    processors: Arc<Mutex<Vec<Box<ProcessesTelemetryMessages>>>>,
    is_running: Arc<AtomicBool>,
    name: Option<String>,
}

impl TelemetryDriver {
    pub fn new<T: Into<String>>(name: T) -> TelemetryDriver {
        let mut driver = TelemetryDriver::default();
        driver.set_name(name);
        driver
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }
}

impl ProcessesTelemetryMessages for TelemetryDriver {
    /// Receive and handle pending operations
    fn process(&mut self, _max: u64) -> u64 {
        0
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let processors = self.processors.lock().unwrap();
        let mut collected = Vec::with_capacity(processors.len());

        for processor in processors.iter() {
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

impl Default for TelemetryDriver {
    fn default() -> TelemetryDriver {
        let driver = TelemetryDriver {
            name: None,
            is_running: Arc::new(AtomicBool::new(true)),
            processors: Arc::new(Mutex::new(Vec::new())),
        };

        start_telemetry_loop(driver.processors.clone(), driver.is_running.clone());

        driver
    }
}

impl AggregatesProcessors for TelemetryDriver {
    fn add_processor(&mut self, processor: Box<ProcessesTelemetryMessages>) {
        self.processors.lock().unwrap().push(processor);
    }
}

impl Drop for TelemetryDriver {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

fn start_telemetry_loop(
    processors: Arc<Mutex<Vec<Box<ProcessesTelemetryMessages>>>>,
    is_running: Arc<AtomicBool>,
) {
    thread::spawn(move || telemetry_loop(&processors, &is_running));
}

fn telemetry_loop(
    processors: &Mutex<Vec<Box<ProcessesTelemetryMessages>>>,
    is_running: &AtomicBool,
) {
    loop {
        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        let started = Instant::now();
        do_a_run(processors);
        let finished = Instant::now();
        let elapsed = finished - started;
        if elapsed < Duration::from_millis(5) {
            thread::sleep(Duration::from_millis(5) - elapsed)
        }
    }
}

fn do_a_run(processors: &Mutex<Vec<Box<ProcessesTelemetryMessages>>>) {
    let mut processors = processors.lock().unwrap();

    for processor in processors.iter_mut() {
        let _ = processor.process(1000);
    }
}
