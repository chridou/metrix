//! The thing that makes it happen... You need it!
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use processor::{AggregatesProcessors, ProcessesTelemetryMessages, ProcessingOutcome};
use snapshot::{ItemKind, Snapshot};
use {Descriptive, PutsSnapshot};
use util;

/// Triggers registered `ProcessesTelemetryMessages` to
/// poll for messages.
///
/// Runs its own background thread. The thread stops once
/// this struct is dropped.
///
/// A `TelemetryDriver` can be 'mounted' into the hierarchy.
/// If done so, it will still poll its children on its own thread
/// independently.
#[derive(Clone)]
pub struct TelemetryDriver {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    processors: Arc<Mutex<Vec<Box<ProcessesTelemetryMessages>>>>,
    snapshooters: Arc<Mutex<Vec<Box<PutsSnapshot>>>>,
    drop_guard: Arc<DropGuard>,
}

struct DropGuard {
    pub is_running: Arc<AtomicBool>,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

impl TelemetryDriver {
    /// Creates a new `TelemetryDriver`.
    ///
    /// `max_observation_age` is the maximum age of an `Observation`
    /// to be taken into account. This is determined by the `timestamp`
    /// field of an `Observation`. `Observations` that are too old are simply dropped.
    /// The default is **60 seconds**.
    pub fn new<T: Into<String>>(
        name: Option<T>,
        max_observation_age: Option<Duration>,
    ) -> TelemetryDriver {
        let is_running = Arc::new(AtomicBool::new(true));

        let driver = TelemetryDriver {
            name: name.map(Into::into),
            title: None,
            description: None,
            drop_guard: Arc::new(DropGuard {
                is_running: is_running.clone(),
            }),
            processors: Arc::new(Mutex::new(Vec::new())),
            snapshooters: Arc::new(Mutex::new(Vec::new())),
        };

        start_telemetry_loop(
            driver.processors.clone(),
            is_running,
            max_observation_age.unwrap_or(Duration::from_secs(60)),
        );

        driver
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_ref().map(|n| &**n)
    }

    pub fn set_name<T: Into<String>>(&mut self, name: T) {
        self.name = Some(name.into())
    }

    pub fn set_title<T: Into<String>>(&mut self, title: T) {
        self.title = Some(title.into())
    }

    pub fn set_description<T: Into<String>>(&mut self, description: T) {
        self.description = Some(description.into())
    }

    pub fn snapshot(&self, descriptive: bool) -> Snapshot {
        let mut outer = Snapshot::default();
        self.put_snapshot(&mut outer, descriptive);
        outer
    }

    fn put_values_into_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        util::put_default_descriptives(self, into, descriptive);
        self.processors
            .lock()
            .unwrap()
            .iter()
            .for_each(|p| p.put_snapshot(into, descriptive));

        self.snapshooters
            .lock()
            .unwrap()
            .iter()
            .for_each(|s| s.put_snapshot(into, descriptive));
    }
}

impl ProcessesTelemetryMessages for TelemetryDriver {
    /// Receive and handle pending operations
    fn process(&mut self, _max: usize, _drop_deadline: Instant) -> ProcessingOutcome {
        ProcessingOutcome::default()
    }
}

impl PutsSnapshot for TelemetryDriver {
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

impl Default for TelemetryDriver {
    fn default() -> TelemetryDriver {
        TelemetryDriver::new::<String>(None, Some(Duration::from_secs(60)))
    }
}

impl AggregatesProcessors for TelemetryDriver {
    fn add_processor<P: ProcessesTelemetryMessages>(&mut self, processor: P) {
        self.processors.lock().unwrap().push(Box::new(processor));
    }

    fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S) {
        self.snapshooters
            .lock()
            .unwrap()
            .push(Box::new(snapshooter));
    }
}

impl Descriptive for TelemetryDriver {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

fn start_telemetry_loop(
    processors: Arc<Mutex<Vec<Box<ProcessesTelemetryMessages>>>>,
    is_running: Arc<AtomicBool>,
    max_observation_age: Duration,
) {
    thread::spawn(move || telemetry_loop(&processors, &is_running, max_observation_age));
}

fn telemetry_loop(
    processors: &Mutex<Vec<Box<ProcessesTelemetryMessages>>>,
    is_running: &AtomicBool,
    max_observation_age: Duration,
) {
    loop {
        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        let started = Instant::now();
        let outcome = do_a_run(processors, 1_000, max_observation_age);
        if outcome.dropped > 0 || outcome.processed > 100 {
            continue;
        }

        let finished = Instant::now();
        let elapsed = finished - started;
        if elapsed < Duration::from_millis(5) {
            thread::sleep(Duration::from_millis(5) - elapsed)
        }
    }
}

fn do_a_run(
    processors: &Mutex<Vec<Box<ProcessesTelemetryMessages>>>,
    max: usize,
    max_observation_age: Duration,
) -> ProcessingOutcome {
    let mut processors = processors.lock().unwrap();

    let mut outcome = ProcessingOutcome::default();

    for processor in processors.iter_mut() {
        let drop_deadline = Instant::now() - max_observation_age;
        outcome.combine_with(&processor.process(max, drop_deadline));
    }

    outcome
}
