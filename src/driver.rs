//! The thing that makes it happen... You need it!
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use instruments::switches::*;
use instruments::*;
use processor::{AggregatesProcessors, ProcessesTelemetryMessages, ProcessingOutcome};
use snapshot::{ItemKind, Snapshot};
use util;
use {Descriptive, PutsSnapshot};

/// Triggers registered `ProcessesTelemetryMessages` to
/// poll for messages.
///
/// Runs its own background thread. The thread stops once
/// this struct is dropped.
///
/// A `TelemetryDriver` can be 'mounted' into the hierarchy.
/// If done so, it will still poll its children on its own thread
/// independently.
///
/// # Optional Metrics
///
/// The driver can be configured to collect metrics on
/// its own activities.
///
/// The metrics will be added to all snapshots
/// under a field named `_metrix` which contains the
/// following fields:
///  
/// * `collections_per_second`: The number of observation collection runs
/// done per second
///
/// * `collection_times_us`: A histogram of the time each observation collection
/// took in microseconds.
///
/// * `observations_processed_per_second`: The number of observations processed
/// per second.
///
/// * `observations_processed_per_collection`: A histogram of the
/// number of observations processed during each run
///
/// * `observations_dropped_per_second`: The number of observations dropped
/// per second. See also `max_observation_age`.
///
/// * `observations_dropped_per_collection`: A histogram of the
/// number of observations dropped during each run. See also
/// `max_observation_age`.
///
/// * `snapshots_per_second`: The number of snapshots taken per second.
///
/// * `snapshots_times_us`: A histogram of the times it took to take a snapshot
/// in microseconds
///
/// * `dropped_observations_alarm`: Will be `true` if observations have been
/// dropped. Will by default stay `true` for 60 seconds once triggered.
///  
/// * `inactivity_alarm`: Will be `true` if no observations have been made for
/// a certain amount of time. The default is 60 seconds.
#[derive(Clone)]
pub struct TelemetryDriver {
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    processors: Arc<Mutex<Vec<Box<ProcessesTelemetryMessages>>>>,
    snapshooters: Arc<Mutex<Vec<Box<PutsSnapshot>>>>,
    drop_guard: Arc<DropGuard>,
    driver_metrics: Option<DriverMetrics>,
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
    /// field of an `Observation`. `Observations` that are too old are simply
    /// dropped. The default is **60 seconds**.
    pub fn new<T: Into<String>>(
        name: Option<T>,
        max_observation_age: Option<Duration>,
    ) -> TelemetryDriver {
        TelemetryDriver::create(name, max_observation_age, false)
    }

    /// Creates a new `TelemetryDriver` which has its own metrics.
    ///
    /// `max_observation_age` is the maximum age of an `Observation`
    /// to be taken into account. This is determined by the `timestamp`
    /// field of an `Observation`. `Observations` that are too old are simply
    /// dropped. The default is **60 seconds**.
    pub fn with_default_metrics<T: Into<String>>(
        name: Option<T>,
        max_observation_age: Option<Duration>,
    ) -> TelemetryDriver {
        TelemetryDriver::create(name, max_observation_age, true)
    }

    fn create<T: Into<String>>(
        name: Option<T>,
        max_observation_age: Option<Duration>,
        with_driver_metrics: bool,
    ) -> TelemetryDriver {
        let is_running = Arc::new(AtomicBool::new(true));

        let driver_metrics = if with_driver_metrics {
            Some(DriverMetrics {
                instruments: Arc::new(Mutex::new(DriverInstruments::default())),
            })
        } else {
            None
        };

        let driver = TelemetryDriver {
            name: name.map(Into::into),
            title: None,
            description: None,
            drop_guard: Arc::new(DropGuard {
                is_running: is_running.clone(),
            }),
            processors: Arc::new(Mutex::new(Vec::new())),
            snapshooters: Arc::new(Mutex::new(Vec::new())),
            driver_metrics: driver_metrics.clone(),
        };

        start_telemetry_loop(
            driver.processors.clone(),
            is_running,
            max_observation_age.unwrap_or(Duration::from_secs(60)),
            driver_metrics,
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
        let started = Instant::now();

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

        if let Some(ref driver_metrics) = self.driver_metrics {
            driver_metrics.update_post_snapshot(started);
            driver_metrics.put_snapshot(into, descriptive);
        }
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
    driver_metrics: Option<DriverMetrics>,
) {
    thread::spawn(move || {
        telemetry_loop(
            &processors,
            &is_running,
            max_observation_age,
            driver_metrics,
        )
    });
}

fn telemetry_loop(
    processors: &Mutex<Vec<Box<ProcessesTelemetryMessages>>>,
    is_running: &AtomicBool,
    max_observation_age: Duration,
    mut driver_metrics: Option<DriverMetrics>,
) {
    let mut last_outcome_logged = Instant::now() - Duration::from_secs(60);
    let mut dropped_since_last_logged = 0usize;
    loop {
        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        let started = Instant::now();
        let outcome = do_a_run(processors, 1_000, max_observation_age);

        dropped_since_last_logged += outcome.dropped;

        if dropped_since_last_logged > 0 && last_outcome_logged.elapsed() > Duration::from_secs(5) {
            log_outcome(dropped_since_last_logged);
            last_outcome_logged = Instant::now();
            dropped_since_last_logged = 0;
        }

        if let Some(ref mut driver_metrics) = driver_metrics {
            driver_metrics.update_post_collection(&outcome, started);
        }

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

#[cfg(feature = "log")]
#[inline]
fn log_outcome(dropped: usize) {
    warn!("{} observations have been dropped.", dropped);
}

#[cfg(not(feature = "log"))]
#[inline]
fn log_outcome(_dropped: usize) {}

#[derive(Clone)]
struct DriverMetrics {
    instruments: Arc<Mutex<DriverInstruments>>,
}

impl DriverMetrics {
    pub fn update_post_collection(&self, outcome: &ProcessingOutcome, collection_started: Instant) {
        self.instruments
            .lock()
            .unwrap()
            .update_post_collection(outcome, collection_started);
    }

    pub fn update_post_snapshot(&self, snapshot_started: Instant) {
        self.instruments
            .lock()
            .unwrap()
            .update_post_snapshot(snapshot_started);
    }

    pub fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        self.instruments
            .lock()
            .unwrap()
            .put_snapshot(into, descriptive);
    }
}

struct DriverInstruments {
    collections_per_second: Meter,
    collection_times_us: Histogram,
    observations_processed_per_second: Meter,
    observations_processed_per_collection: Histogram,
    observations_dropped_per_second: Meter,
    observations_dropped_per_collection: Histogram,
    snapshots_per_second: Meter,
    snapshots_times_us: Histogram,
    dropped_observations_alarm: StaircaseTimer,
    inactivity_alarm: NonOccurrenceIndicator,
}

impl Default for DriverInstruments {
    fn default() -> Self {
        DriverInstruments {
            collections_per_second: Meter::new_with_defaults("collections_per_second"),
            collection_times_us: Histogram::new_with_defaults("collection_times_us"),
            observations_processed_per_second: Meter::new_with_defaults(
                "observations_processed_per_second",
            ),
            observations_processed_per_collection: Histogram::new_with_defaults(
                "observations_processed_per_collection",
            ),
            observations_dropped_per_second: Meter::new_with_defaults(
                "observations_dropped_per_second",
            ),
            observations_dropped_per_collection: Histogram::new_with_defaults(
                "observations_dropped_per_collection",
            ),
            snapshots_per_second: Meter::new_with_defaults("snapshots_per_second"),
            snapshots_times_us: Histogram::new_with_defaults("snapshots_times_us"),
            dropped_observations_alarm: StaircaseTimer::new_with_defaults(
                "dropped_observations_alarm",
            ),
            inactivity_alarm: NonOccurrenceIndicator::new_with_defaults("inactivity_alarm"),
        }
    }
}

impl DriverInstruments {
    pub fn update_post_collection(
        &mut self,
        outcome: &ProcessingOutcome,
        collection_started: Instant,
    ) {
        let now = Instant::now();
        self.collections_per_second
            .update(&Update::Observation(now));
        self.collection_times_us
            .update(&Update::ObservationWithValue(
                duration_to_micros(now - collection_started),
                now,
            ));
        if outcome.processed > 0 {
            self.observations_processed_per_second
                .update(&Update::Observations(outcome.processed as u64, now));
            self.observations_processed_per_collection
                .update(&Update::ObservationWithValue(outcome.processed as u64, now));
        }
        if outcome.dropped > 0 {
            self.observations_dropped_per_second
                .update(&Update::Observations(outcome.dropped as u64, now));
            self.observations_dropped_per_collection
                .update(&Update::ObservationWithValue(outcome.dropped as u64, now));
            self.dropped_observations_alarm
                .update(&Update::Observation(now));
        }
        self.inactivity_alarm.update(&Update::Observation(now));
    }

    pub fn update_post_snapshot(&mut self, snapshot_started: Instant) {
        let now = Instant::now();
        self.snapshots_per_second.update(&Update::Observation(now));
        self.snapshots_times_us
            .update(&Update::ObservationWithValue(
                duration_to_micros(now - snapshot_started),
                now,
            ));
    }

    pub fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        let mut container = Snapshot::default();
        self.collections_per_second
            .put_snapshot(&mut container, descriptive);
        self.collection_times_us
            .put_snapshot(&mut container, descriptive);
        self.observations_processed_per_second
            .put_snapshot(&mut container, descriptive);
        self.observations_processed_per_collection
            .put_snapshot(&mut container, descriptive);
        self.observations_dropped_per_second
            .put_snapshot(&mut container, descriptive);
        self.observations_dropped_per_collection
            .put_snapshot(&mut container, descriptive);
        self.snapshots_per_second
            .put_snapshot(&mut container, descriptive);
        self.snapshots_times_us
            .put_snapshot(&mut container, descriptive);
        self.dropped_observations_alarm
            .put_snapshot(&mut container, descriptive);
        self.inactivity_alarm
            .put_snapshot(&mut container, descriptive);

        into.items
            .push(("_metrix".into(), ItemKind::Snapshot(container)));
    }
}

#[inline]
fn duration_to_micros(d: Duration) -> u64 {
    let nanos = (d.as_secs() * 1_000_000_000) + (d.subsec_nanos() as u64);
    nanos / 1000
}
