//! # metrix
//!
//! Metrics for monitoring applications and alerting.
//!
//! ## Goal
//!
//! Applications/services can have a lot of metrics and one of the greatest
//! challenges is organizing them. This is what `metrix` tries to help with.
//!
//! **Metrix** does not aim for providing exact numbers and aims for
//! applications monitoring only.
//!
//! This crate is in a very **early** stage and the API might still change.
//! There may be backends provided for monitoring solutions in the future
//! but currently only a snapshot that can be
//! serialized to JSON is provided.
//!
//! ## How does it work
//!
//! **Metrix** is based on observations collected while running your
//! application. These observations will then be sent to a backend where
//! the actual metrics(counters etc.) are updated. For the metrics configured
//! a snapshot can be queried.
//!
//! The primary focus of **metrix** is to organize these metrics. There are
//! several building blocks available. Most of them can have a name that will
//! then be part of a path within a snapshot.
//!
//! ### Labels
//!
//! Labels link observations to panels. Labels can be of any type that
//! implements `Clone + Eq + Send + 'static`. An `enum` is a good choice for a
//! label.
//!
//! ### Observations
//!
//! An observation is made somewhere within your application. When an
//! observation is sent to the backend it must have a label attached. This label
//! is then matched against the label of a panel to determine whether an
//! observation is handled for updating or not.
//!
//! ### Instruments
//!
//! Instruments are gauges, meters, etc. An instrument gets updated by an
//! observation where an update is meaningful. Instruments are grouped by
//! `Panel`s.
//!
//! You can find instruments in the module `instruments`.
//!
//! ### Panels
//!
//! A `Panel` groups instruments under same same label. So each instrument
//! within a panel will be updated by observations that have the same label as
//! the panel.
//!
//! Lets say you defined a label `OutgoingRequests`. If you are interested
//! in the request rate and the latencies. You would then create a panel with a
//! label `OutgoingRequests` and add a histogram and a meter.
//!
//! ### Cockpit
//!
//! A cockpit aggregates multiple `Panel`s. A cockpit can be used to monitor
//! different tasks/parts of a component or workflow. A cockpit
//! is bound to a label type.
//!
//! An example can be that you have service component that calls an external
//! HTTP client. You could be interested in successful calls and failed calls
//! individually. So for both cases you would create a value for your label
//! and then add two panels to the cockpit.
//!
//! Cockpits are in the module `cockpit`.
//!
//! ### Processors
//!
//! The most important processor is the `TelemetryProcessor`. It has
//! a label type as a type parameter and consist of a `TelemetryTransmitter`
//! that sends observations to the backend(used within your app)
//! and the actual `TelemetryProcessor` that forms the backend and
//! processes observations. The `TelemetryProcessor`
//! can **own** several cockpits for a label type.
//!
//! There is also a `ProcessorMount` that is label agnostic and can group
//! several processors. It can also have a name that will be included in the
//! snapshot.
//!
//! The processors can be found the module `processor`.
//!
//! ### Driver
//!
//! The driver **owns** processors and asks the **owned** processors
//! to process their messages. You need to add your processors to
//! a driver to start the machinery. A driver is also a processor
//! which means it can have a name and it can also be part of another
//! hierarchy.
//!
//! Each driver has its own thread for polling its processors
//! so even when attached to another
//! hierarchy all processors registered with the driver will only
//! be driven by that driver.
//!
//!
//! ## Contributing
//!
//! Contributing is welcome. Criticism is also welcome!
//!
//! ## License
//!
//! Metrix is primarily distributed under the terms of
//! both the MIT license and the Apache License (Version 2.0).
//!
//! Copyright (c) 2018 Christian Douven
//!
#[cfg(feature = "log")]
#[macro_use]
extern crate log;

use std::time::{Duration, Instant};

use snapshot::Snapshot;

use cockpit::Cockpit;
use instruments::Panel;
use processor::TelemetryMessage;

pub use observation::*;
pub use processor::AggregatesProcessors;

pub mod attached_mount;
pub mod cockpit;
pub mod driver;
pub mod instruments;
mod observation;
pub mod processor;
pub mod snapshot;

pub(crate) mod util;

/// Something that can react on `Observation`s where
/// the `Label` is the type of the label.
///
/// You can use this to implement your own metrics.
pub trait HandlesObservations: PutsSnapshot + Send + 'static {
    type Label: Send + 'static;
    fn handle_observation(&mut self, observation: &Observation<Self::Label>) -> usize;
}

/// Increments a value by one (e.g. in a `Gauge`)
#[derive(Debug, Copy, Clone)]
pub struct Increment;
/// Increments a value by the given amount (e.g. in a `Gauge`)
#[derive(Debug, Copy, Clone)]
pub struct IncrementBy(pub u32);
/// Decrements a value by one (e.g. in a `Gauge`)
#[derive(Debug, Copy, Clone)]
pub struct Decrement;
/// Decrements a value by the given amount (e.g. in a `Gauge`)
#[derive(Debug, Copy, Clone)]
pub struct DecrementBy(pub u32);
/// Changes a value by the given amount (e.g. in a `Gauge`)
#[derive(Debug, Copy, Clone)]
pub struct ChangeBy(pub i64);

/// Transmits telemetry data to the backend.
///
/// Implementors should transfer `Observations` to
/// a backend and manipulate the instruments there to not
/// to interfere to much with the actual task being measured/observed
pub trait TransmitsTelemetryData<L> {
    /// Transit an observation to the backend.
    fn transmit(&self, observation: Observation<L>) -> &Self;

    /// Observed `count` occurrences at time `timestamp`
    ///
    /// Convenience method. Simply calls `transmit`
    fn observed(&self, label: L, count: u64, timestamp: Instant) -> &Self {
        self.transmit(Observation::Observed {
            label,
            count,
            timestamp,
        })
    }

    /// Observed one occurrence at time `timestamp`
    ///
    /// Convenience method. Simply calls `transmit`
    fn observed_one(&self, label: L, timestamp: Instant) -> &Self {
        self.transmit(Observation::ObservedOne { label, timestamp })
    }

    /// Observed one occurrence with value `value` at time `timestamp`
    ///
    /// Convenience method. Simply calls `transmit`
    fn observed_one_value<V: Into<ObservedValue>>(
        &self,
        label: L,
        value: V,
        timestamp: Instant,
    ) -> &Self {
        self.transmit(Observation::ObservedOneValue {
            label,
            value: value.into(),
            timestamp,
        })
    }

    /// Sends a `Duration` as an observed value observed at `timestamp`.
    /// The `Duration` is converted to nanoseconds.
    fn observed_duration(&self, label: L, duration: Duration, timestamp: Instant) -> &Self {
        self.observed_one_value(label, duration, timestamp)
    }

    /// Observed `count` occurrences at now.
    ///
    /// Convenience method. Simply calls `observed` with
    /// the current timestamp.
    fn observed_now(&self, label: L, count: u64) -> &Self {
        self.observed(label, count, Instant::now())
    }

    /// Observed one occurrence now
    ///
    /// Convenience method. Simply calls `observed_one` with
    /// the current timestamp.
    fn observed_one_now(&self, label: L) -> &Self {
        self.observed_one(label, Instant::now())
    }

    /// Observed one occurrence with value `value` now
    ///
    /// Convenience method. Simply calls `observed_one_value` with
    /// the current timestamp.
    fn observed_one_value_now<V: Into<ObservedValue>>(&self, label: L, value: V) -> &Self {
        self.observed_one_value(label, value, Instant::now())
    }

    /// Sends a `Duration` as an observed value observed with the current
    /// timestamp.
    ///
    /// The `Duration` is converted to nanoseconds internally.
    fn observed_one_duration_now(&self, label: L, duration: Duration) -> &Self {
        self.observed_duration(label, duration, Instant::now())
    }

    /// Measures the time from `from` until now.
    ///
    /// The resulting duration is an observed value
    /// with the measured duration in nanoseconds.
    fn measure_time(&self, label: L, from: Instant) -> &Self {
        let now = Instant::now();
        if from <= now {
            self.observed_duration(label, now - from, now);
        }

        self
    }

    /// Add a handler.
    fn add_handler<H: HandlesObservations<Label = L>>(&self, handler: H) -> &Self
    where
        L: Send + 'static;

    /// Add a `Copckpit`
    ///
    /// If the cockpit has a name and another cockpit with
    /// the same name is already present the cockpit will
    /// not be added.
    fn add_cockpit(&self, cockpit: Cockpit<L>) -> &Self;

    fn remove_cockpit<T: Into<String>>(&self, name: T) -> &Self;

    /// Add a `Panel` to a `Cockpit` if that `Cockpit` has the
    /// given name.
    fn add_panel_to_cockpit<T: Into<String>>(&self, cockpit_name: T, panel: Panel<L>) -> &Self;

    /// Removes the panel with the given name from a cockpit
    /// with the given name.
    ///
    /// This means the cockpit and the panel must have a name set.
    fn remove_panel_from_cockpit<U: Into<String>, V: Into<String>>(
        &self,
        cockpit_name: U,
        panel_name: V,
    ) -> &Self;
}

/// Transmits `Observation`s to the backend
///
/// It is important that the corresponding `TelemetryProcessor`
/// gets mounted on a driver soon
/// since otherwise the internal queue will get flooded
/// with unprocessed observations
#[derive(Clone)]
pub struct TelemetryTransmitter<L> {
    sender: crossbeam_channel::Sender<TelemetryMessage<L>>,
    // True if we want to block in case the queue
    // is full. Has no effect on unbounded queue
    use_send: bool,
}

impl<L: Send> TelemetryTransmitter<L> {
    fn send(&self, msg: TelemetryMessage<L>) -> &Self {
        if self.use_send {
            if let Err(err) = self.sender.send(msg) {
                util::log_warning(format!("failed to send telemetry message: {}", err))
            }
        } else if let Err(err) = self.sender.try_send(msg) {
            util::log_warning(format!("failed to send telemetry message: {}", err))
        }
        self
    }

    /// Returns `true` if the internal queue is full.
    ///
    /// Always `false` on an unbounded queue
    pub fn is_queue_full(&self) -> bool {
        self.sender.is_full()
    }

    /// Returns `true` if the internal queue is empty.
    ///
    /// Always `true` on an unbounded queue
    pub fn is_queue_empty(&self) -> bool {
        self.sender.is_empty()
    }

    pub fn queue_size(&self) -> usize {
        self.sender.len()
    }

    /// Returns the capacity of the internal queue.
    ///
    /// `None` on an unbounded queue
    pub fn queue_capacity(&self) -> Option<usize> {
        self.sender.capacity()
    }
}

impl<L: Send + 'static> TransmitsTelemetryData<L> for TelemetryTransmitter<L> {
    fn transmit(&self, observation: Observation<L>) -> &Self {
        self.send(TelemetryMessage::Observation(observation))
    }

    fn add_handler<H: HandlesObservations<Label = L>>(&self, handler: H) -> &Self {
        self.send(TelemetryMessage::AddHandler(Box::new(handler)))
    }

    fn add_cockpit(&self, cockpit: Cockpit<L>) -> &Self {
        self.send(TelemetryMessage::AddCockpit(cockpit))
    }

    fn remove_cockpit<T: Into<String>>(&self, name: T) -> &Self {
        self.send(TelemetryMessage::RemoveCockpit(name.into()))
    }

    fn add_panel_to_cockpit<T: Into<String>>(&self, cockpit_name: T, panel: Panel<L>) -> &Self {
        self.send(TelemetryMessage::AddPanelToCockpit {
            cockpit_name: cockpit_name.into(),
            panel,
        })
    }

    fn remove_panel_from_cockpit<U: Into<String>, V: Into<String>>(
        &self,
        cockpit_name: U,
        panel_name: V,
    ) -> &Self {
        self.send(TelemetryMessage::RemovePanelFromCockpit {
            cockpit_name: cockpit_name.into(),
            panel_name: panel_name.into(),
        })
    }
}

/// Something that has a title and a description
///
/// This is mostly useful for snapshots. When a `Snapshot`
/// is taken there is usually a parameter `descriptive`
/// that determines whether title and description should
/// be part of a `Snapshot`. See also `PutsSnapshot`.
pub trait Descriptive {
    fn title(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        None
    }
}

/// Implementors are able to write their current data into given `Snapshot`.
///
/// Guidelines for writing snapshots:
///
/// * A `PutsSnapshot` that has a name should create a new sub snapshot
/// and add its values there
///
/// * A `PutsSnapshot` that does not have a name should add its values
/// directly to the given snapshot
///
/// * When `descriptive` is set to `true` `PutsSnapshot` should put
/// its `title` and `description` into the same `Snapshot` it put
/// its values(exception: instruments) thereby not overwriting already
/// existing descriptions so that the more general top level ones survive.
///
/// * When `descriptive` is set to `true` on an instrument the instrument
/// should put its description into the snapshot it got passed therby adding the
/// suffixes "_title" and "_description" to its name.
///
/// Implementors of this trait can be added to almost all components via
/// the `add_snapshooter` method which is also defined on trait
/// `AggregatesProcessors`.
pub trait PutsSnapshot: Send + 'static {
    /// Puts the current snapshot values into the given `Snapshot` thereby
    /// following the guidelines of `PutsSnapshot`.
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool);
}
