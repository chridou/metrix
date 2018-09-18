//! The thing that makes it happen... You need it!
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{self, Receiver as CrossbeamReceiver, Sender as CrossbeamSender};
use futures::future::Future;
use futures::sync::oneshot;

use instruments::switches::*;
use instruments::*;
use processor::{
    AggregatesProcessors, ProcessesTelemetryMessages, ProcessingOutcome, ProcessingStrategy,
};
use snapshot::{ItemKind, Snapshot};
use util;
use {Descriptive, PutsSnapshot};

/// A Builder for a `TelemetryDriver`
pub struct DriverBuilder {
    /// An optional name that will also group the metrics under the name
    ///
    /// Default is `None`
    pub name: Option<String>,
    /// A title to be added when a `Snapshot` with descriptions is created
    ///
    /// Default is `None`
    pub title: Option<String>,
    /// A description to be added when a `Snapshot` with descriptions is created
    ///
    /// Default is `None`
    pub description: Option<String>,
    /// Sets the `ProcessingStrategy`
    /// dropped. The default is **60 seconds**.
    pub processing_strategy: ProcessingStrategy,
    /// If true metrics for the `TelemetryDriver` will be added to the
    /// generated `Snapshot`
    ///
    /// Default is `true`
    pub with_driver_metrics: bool,
}

impl DriverBuilder {
    pub fn new<T: Into<String>>(name: T) -> DriverBuilder {
        let mut me = Self::default();
        me.name = Some(name.into());
        me
    }

    pub fn set_name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn set_title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn set_description<T: Into<String>>(mut self, description: T) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn set_processing_strategy(mut self, processing_strategy: ProcessingStrategy) -> Self {
        self.processing_strategy = processing_strategy;
        self
    }

    pub fn set_driver_metrics(mut self, enabled: bool) -> Self {
        self.with_driver_metrics = enabled;
        self
    }

    pub fn build(self) -> TelemetryDriver {
        TelemetryDriver::new(
            self.name,
            self.title,
            self.description,
            self.processing_strategy,
            self.with_driver_metrics,
        )
    }
}

impl Default for DriverBuilder {
    fn default() -> Self {
        Self {
            name: None,
            title: None,
            description: None,
            processing_strategy: ProcessingStrategy::default(),
            with_driver_metrics: true,
        }
    }
}

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
    descriptives: Descriptives,
    drop_guard: Arc<DropGuard>,
    sender: CrossbeamSender<DriverMessage>,
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
    pub fn new(
        name: Option<String>,
        title: Option<String>,
        description: Option<String>,
        processing_strategy: ProcessingStrategy,
        with_driver_metrics: bool,
    ) -> TelemetryDriver {
        let is_running = Arc::new(AtomicBool::new(true));

        let driver_metrics = if with_driver_metrics {
            Some(DriverMetrics {
                instruments: DriverInstruments::default(),
            })
        } else {
            None
        };

        let (sender, receiver) = crossbeam_channel::unbounded();

        let mut descriptives = Descriptives::default();
        descriptives.name = name;
        descriptives.title = title;
        descriptives.description = description;

        let driver = TelemetryDriver {
            descriptives: descriptives.clone(),
            drop_guard: Arc::new(DropGuard {
                is_running: is_running.clone(),
            }),
            sender,
        };

        start_telemetry_loop(
            descriptives,
            is_running,
            processing_strategy,
            driver_metrics,
            receiver,
        );

        driver
    }

    /// Gets the name of this driver
    pub fn name(&self) -> Option<&str> {
        self.descriptives.name.as_ref().map(|n| &**n)
    }

    /// Changes the `ProcessingStrategy`
    pub fn change_processing_stragtegy(&self, strategy: ProcessingStrategy) {
        let _ = self
            .sender
            .send(DriverMessage::SetProcessingStrategy(strategy));
    }

    /// Pauses processing of observations.
    pub fn pause(&self) {
        let _ = self.sender.send(DriverMessage::Pause);
    }

    /// Resumes processing of observations
    pub fn resume(&self) {
        let _ = self.sender.send(DriverMessage::Resume);
    }

    pub fn snapshot(&self, descriptive: bool) -> Result<Snapshot, GetSnapshotError> {
        let snapshot = Snapshot::default();
        let (tx, rx) = crossbeam_channel::unbounded();
        let _ = self
            .sender
            .send(DriverMessage::GetSnapshotSync(snapshot, tx, descriptive));
        if let Some(snapshot) = rx.recv() {
            Ok(snapshot)
        } else {
            Err(GetSnapshotError)
        }
    }

    pub fn snapshot_async(
        &self,
        descriptive: bool,
    ) -> impl Future<Item = Snapshot, Error = GetSnapshotError> + Send + 'static {
        let snapshot = Snapshot::default();
        let (tx, rx) = oneshot::channel();
        let _ = self
            .sender
            .send(DriverMessage::GetSnapshotAsync(snapshot, tx, descriptive));
        rx.map_err(|_| GetSnapshotError)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GetSnapshotError;

impl ::std::error::Error for GetSnapshotError {
    fn description(&self) -> &str {
        "Could not create a snapshot"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

impl fmt::Display for GetSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", ::std::error::Error::description(self))
    }
}

impl ProcessesTelemetryMessages for TelemetryDriver {
    /// Receive and handle pending operations
    fn process(&mut self, _max: usize, _strategy: ProcessingStrategy) -> ProcessingOutcome {
        ProcessingOutcome::default()
    }
}

impl PutsSnapshot for TelemetryDriver {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        if let Ok(snapshot) = self.snapshot(descriptive) {
            snapshot
                .items
                .into_iter()
                .for_each(|(k, v)| into.push(k, v));
        }
    }
}

impl Default for TelemetryDriver {
    fn default() -> TelemetryDriver {
        TelemetryDriver::new(None, None, None, ProcessingStrategy::default(), true)
    }
}

impl AggregatesProcessors for TelemetryDriver {
    fn add_processor<P: ProcessesTelemetryMessages>(&mut self, processor: P) {
        let _ = self
            .sender
            .send(DriverMessage::AddProcessor(Box::new(processor)));
    }

    fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S) {
        let _ = self
            .sender
            .send(DriverMessage::AddSnapshooter(Box::new(snapshooter)));
    }
}

impl Descriptive for TelemetryDriver {
    fn title(&self) -> Option<&str> {
        self.descriptives.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.descriptives.description.as_ref().map(|n| &**n)
    }
}

fn start_telemetry_loop(
    descriptives: Descriptives,
    is_running: Arc<AtomicBool>,
    processing_strategy: ProcessingStrategy,
    driver_metrics: Option<DriverMetrics>,
    receiver: CrossbeamReceiver<DriverMessage>,
) {
    thread::spawn(move || {
        telemetry_loop(
            descriptives,
            &is_running,
            processing_strategy,
            driver_metrics,
            receiver,
        )
    });
}

enum DriverMessage {
    AddProcessor(Box<dyn ProcessesTelemetryMessages>),
    AddSnapshooter(Box<dyn PutsSnapshot>),
    GetSnapshotSync(Snapshot, CrossbeamSender<Snapshot>, bool),
    GetSnapshotAsync(Snapshot, oneshot::Sender<Snapshot>, bool),
    SetProcessingStrategy(ProcessingStrategy),
    Pause,
    Resume,
}

fn telemetry_loop(
    descriptives: Descriptives,
    is_running: &AtomicBool,
    processing_strategy: ProcessingStrategy,
    mut driver_metrics: Option<DriverMetrics>,
    receiver: CrossbeamReceiver<DriverMessage>,
) {
    let mut last_outcome_logged = Instant::now() - Duration::from_secs(60);
    let mut dropped_since_last_logged = 0usize;

    let mut processors: Vec<Box<dyn ProcessesTelemetryMessages>> = Vec::new();
    let mut snapshooters: Vec<Box<dyn PutsSnapshot>> = Vec::new();

    let mut processing_stragtegy = processing_strategy;

    let mut paused = false;

    loop {
        if !is_running.load(Ordering::Relaxed) {
            break;
        }

        if let Some(message) = receiver.try_recv() {
            match message {
                DriverMessage::AddProcessor(processor) => processors.push(processor),
                DriverMessage::AddSnapshooter(snapshooter) => snapshooters.push(snapshooter),
                DriverMessage::GetSnapshotSync(mut snapshot, back_channel, descriptive) => {
                    put_values_into_snapshot(
                        &mut snapshot,
                        &processors,
                        &snapshooters,
                        driver_metrics.as_mut(),
                        &descriptives,
                        descriptive,
                    );
                    let _ = back_channel.send(snapshot);
                }
                DriverMessage::GetSnapshotAsync(mut snapshot, back_channel, descriptive) => {
                    put_values_into_snapshot(
                        &mut snapshot,
                        &processors,
                        &snapshooters,
                        driver_metrics.as_mut(),
                        &descriptives,
                        descriptive,
                    );
                    let _ = back_channel.send(snapshot);
                }
                DriverMessage::SetProcessingStrategy(strategy) => {
                    log_info(&format!("Processing strategy changed to {:?}", strategy));
                    processing_stragtegy = strategy
                }
                DriverMessage::Pause => {
                    log_info("pausing");
                    paused = true
                }
                DriverMessage::Resume => {
                    paused = {
                        log_info("resuming");
                        false
                    }
                }
            }
        }

        if paused {
            thread::sleep(Duration::from_millis(50));
            continue;
        }

        let started = Instant::now();
        let outcome = do_a_run(&mut processors, 1_000, processing_stragtegy);

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
        if elapsed < Duration::from_millis(10) {
            thread::sleep(Duration::from_millis(10) - elapsed)
        }
    }
}

fn do_a_run(
    processors: &mut [Box<dyn ProcessesTelemetryMessages>],
    max: usize,
    strategy: ProcessingStrategy,
) -> ProcessingOutcome {
    let mut outcome = ProcessingOutcome::default();

    for processor in processors.iter_mut() {
        outcome.combine_with(&processor.process(max, strategy));
    }

    outcome
}

fn put_values_into_snapshot(
    into: &mut Snapshot,
    processors: &[Box<dyn ProcessesTelemetryMessages>],
    snapshooters: &[Box<dyn PutsSnapshot>],
    driver_metrics: Option<&mut DriverMetrics>,
    descriptives: &Descriptives,
    descriptive: bool,
) {
    let started = Instant::now();

    if let Some(ref name) = descriptives.name {
        let mut new_level = Snapshot::default();
        add_snapshot_values(
            &mut new_level,
            &processors,
            &snapshooters,
            driver_metrics,
            &descriptives,
            descriptive,
            started,
        );
        into.items
            .push((name.clone(), ItemKind::Snapshot(new_level)));
    } else {
        add_snapshot_values(
            into,
            &processors,
            &snapshooters,
            driver_metrics,
            &descriptives,
            descriptive,
            started,
        );
    }
}

fn add_snapshot_values(
    into: &mut Snapshot,
    processors: &[Box<dyn ProcessesTelemetryMessages>],
    snapshooters: &[Box<dyn PutsSnapshot>],
    driver_metrics: Option<&mut DriverMetrics>,
    descriptives: &Descriptives,
    descriptive: bool,
    started: Instant,
) {
    util::put_default_descriptives(descriptives, into, descriptive);
    processors
        .iter()
        .for_each(|p| p.put_snapshot(into, descriptive));

    snapshooters
        .iter()
        .for_each(|s| s.put_snapshot(into, descriptive));

    if let Some(driver_metrics) = driver_metrics {
        driver_metrics.update_post_snapshot(started);
        driver_metrics.put_snapshot(into, descriptive);
    }
}

#[derive(Clone)]
struct Descriptives {
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl Default for Descriptives {
    fn default() -> Self {
        Self {
            name: None,
            title: None,
            description: None,
        }
    }
}

impl Descriptive for Descriptives {
    fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|n| &**n)
    }

    fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|n| &**n)
    }
}

#[cfg(feature = "log")]
#[inline]
fn log_outcome(dropped: usize) {
    warn!("{} observations have been dropped.", dropped);
}

#[cfg(not(feature = "log"))]
#[inline]
fn log_outcome(_dropped: usize) {}

#[cfg(feature = "log")]
#[inline]
fn log_info(message: &str) {
    info!("{}", message);
}

#[cfg(not(feature = "log"))]
#[inline]
fn log_info(_message: &str) {}

struct DriverMetrics {
    instruments: DriverInstruments,
}

impl DriverMetrics {
    pub fn update_post_collection(
        &mut self,
        outcome: &ProcessingOutcome,
        collection_started: Instant,
    ) {
        self.instruments
            .update_post_collection(outcome, collection_started);
    }

    pub fn update_post_snapshot(&mut self, snapshot_started: Instant) {
        self.instruments.update_post_snapshot(snapshot_started);
    }

    pub fn put_snapshot(&mut self, into: &mut Snapshot, descriptive: bool) {
        self.instruments.put_snapshot(into, descriptive);
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
