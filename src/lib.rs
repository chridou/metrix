//! # metrix
//!
//! [![crates.io](https://img.shields.io/crates/v/metrix.svg)]
//! (https://crates.io/crates/metrix)
//! [![docs.rs](https://docs.rs/metrix/badge.svg)]
//! (https://docs.rs/metrix)
//! [![downloads](https://img.shields.io/crates/d/metrix.svg)]
//! (https://crates.io/crates/metrix)
//! [![Build Status](https://travis-ci.org/chridou/metrix.svg?branch=master)]
//! (https://travis-ci.org/chridou/metrix)
//! [![license-mit](http://img.shields.io/badge/license-MIT-blue.svg)]
//! (https://github.com/chridou/metrix/blob/master/LICENSE-MIT)
//! [![license-apache](http://img.shields.io/badge/license-APACHE-blue.svg)]
//! (https://github.com/chridou/metrix/blob/master/LICENSE-APACHE)
//!
//!
//! Metrics for monitoring applications and alerting.
//!
//! ## Goal
//!
//! Applications/services can have a lot of metrics and one of the greatest challenges is
//! organizing them. This is what `metrix` tries to help with.
//!
//! **Metrix** does not aim for providing exact numbers and aims for applications monitoring
//! only.
//!
//! This crate is in a very **early** stage and the API might still change. There may be
//! backends provided for monitoring solutions in the future
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
//! The primary focus of **metrix** is to organize these metrics. There are several
//! building blocks available. Most of them can have a name that will then be part
//! of a path within a snapshot.
//!
//! ### Labels
//!
//! Labels link observations to panels. Labels can be of any type that implements
//! `Clone + Eq + Send + 'static`. An `enum` is a good choice for a label.
//!
//! ### Observations
//!
//! An abservation is made somewhere within your application. When an observation
//! is sent to the backend it must have a label attached. This label
//! is then matched against the label of a panel to determine whether an observation is
//! handled for updating or not.
//!
//! ### Instruments
//!
//! Instruments are gauges, meters, etc. An instrument gets updated by an observation
//! where an update is meaningful. Instruments are grouped by `Panel`s.
//!
//! You can find instruments in the module `instruments`.
//!
//! ### Panels
//!
//! A `Panel` groups instruments under same same label. So each instrument within
//! a panel will be updated by observations that have the same label as the panel.
//!
//! Lets say you defined a label `OutgoingRequests`. If you are interested
//! in the request rate and the latencies. You would then create a panel with a
//! label `OutgoingRequests` and add a histogram and a meter.
//!
//! ### Cockpit
//!
//! A cockpit aggregates multiple `Panel`s. A cockpit can be used to monitor
//! different tasks/parts of a component or worklflow. A cockpit
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

extern crate exponential_decay_histogram;
extern crate json;
extern crate metrics;

use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use snapshot::Snapshot;

use processor::TelemetryMessage;
use instruments::{Panel, ValueScaling};
use cockpit::{Cockpit, HandlesObservations};

pub mod instruments;
pub mod snapshot;
pub mod processor;
pub mod driver;
pub mod cockpit;
pub(crate) mod util;

/// An observation that has been made.
///
/// Be aware that not all instruments handle all
/// observations or values.
/// E.g. a `Meter` does not take the `value` of
/// an `Observation::ObservedOneValue` into account but
/// simply counts the observation as one occurence.
#[derive(Clone)]
pub enum Observation<L> {
    /// Observed many occurances at th given timestamp
    Observed {
        label: L,
        count: u64,
        timestamp: Instant,
    },
    /// Observed one occurrence at the given timestamp
    ObservedOne { label: L, timestamp: Instant },
    /// Observed one occurence with a value at a given timestamp.
    ObservedOneValue {
        label: L,
        value: u64,
        timestamp: Instant,
    },
}

impl<L> Observation<L>
where
    L: Clone,
{
    /// Extracts the label `L` from an observation.
    pub fn label(&self) -> &L {
        match *self {
            Observation::Observed { ref label, .. } => label,
            Observation::ObservedOne { ref label, .. } => label,
            Observation::ObservedOneValue { ref label, .. } => label,
        }
    }

    /// Scale by the given `ValueScaling`
    ///
    /// This will clone the `Observation`
    pub fn scaled(&self, scaling: ValueScaling) -> Observation<L> {
        let mut cloned = (*self).clone();

        match cloned {
            Observation::ObservedOneValue { ref mut value, .. } => match scaling {
                ValueScaling::NanosToMillis => *value = *value / 1_000_000,
                ValueScaling::NanosToMicros => *value = *value / 1_000,
            },
            _ => (),
        }

        cloned
    }
}

impl<L> Observation<L> {
    pub fn timestamp(&self) -> Instant {
        match *self {
            Observation::Observed { timestamp, .. } => timestamp,
            Observation::ObservedOne { timestamp, .. } => timestamp,
            Observation::ObservedOneValue { timestamp, .. } => timestamp,
        }
    }
}

/// Transmits telemetry data to the backend.
///
/// Implementors should tranfer `Observations` to
/// a backend and manipulate the instruments there to not
/// to interfere to much with the actual task being measured/observed
pub trait TransmitsTelemetryData<L> {
    /// Transit an observation to the backend.
    fn transmit(&self, observation: Observation<L>);

    /// Observed `count` occurences at time `timestamp`
    ///
    /// Convinience method. Simply calls `transmit`
    fn observed(&self, label: L, count: u64, timestamp: Instant) {
        self.transmit(Observation::Observed {
            label,
            count,
            timestamp,
        })
    }

    /// Observed one occurence at time `timestamp`
    ///
    /// Convinience method. Simply calls `transmit`
    fn observed_one(&self, label: L, timestamp: Instant) {
        self.transmit(Observation::ObservedOne { label, timestamp })
    }

    /// Observed one occurence with value `value` at time `timestamp`
    ///
    /// Convinience method. Simply calls `transmit`
    fn observed_one_value(&self, label: L, value: u64, timestamp: Instant) {
        self.transmit(Observation::ObservedOneValue {
            label,
            value,
            timestamp,
        })
    }

    /// Sends a `Duration` as an observed value observed at `timestamp`.
    /// The `Duration` is converted to nanoseconds.
    fn observed_duration(&self, label: L, duration: Duration, timestamp: Instant) {
        let nanos = (duration.as_secs() * 1_000_000_000) + (duration.subsec_nanos() as u64);
        self.observed_one_value(label, nanos, timestamp)
    }

    /// Observed `count` occurences at now.
    ///
    /// Convinience method. Simply calls `observed` with
    /// the current timestamp.
    fn observed_now(&self, label: L, count: u64) {
        self.observed(label, count, Instant::now())
    }

    /// Observed one occurence now
    ///
    /// Convinience method. Simply calls `observed_one` with
    /// the current timestamp.
    fn observed_one_now(&self, label: L) {
        self.observed_one(label, Instant::now())
    }

    /// Observed one occurence with value `value` now
    ///
    /// Convinience method. Simply calls `observed_one_value` with
    /// the current timestamp.
    fn observed_one_value_now(&self, label: L, value: u64) {
        self.observed_one_value(label, value, Instant::now())
    }

    /// Sends a `Duration` as an observed value observed with the current
    /// timestamp.
    ///
    /// The `Duration` is converted to nanoseconds.
    fn observed_one_duration_now(&self, label: L, duration: Duration) {
        self.observed_duration(label, duration, Instant::now());
    }

    /// Measures the time from `from` until now.
    ///
    /// The resultiong duration is an observed value
    /// with the measured duration in nanoseconds.
    fn measure_time(&self, label: L, from: Instant) {
        let now = Instant::now();
        if from <= now {
            self.observed_duration(label, now - from, now)
        }
    }

    /// Add a handler.
    fn add_handler(&self, handler: Box<HandlesObservations<Label = L>>);

    /// Add a `Copckpit`
    fn add_cockpit(&self, cockpit: Cockpit<L>);

    /// Add a `Panel` to a `Cockpit` if that `Cockpit` has the
    /// given name.
    fn add_panel_to_cockpit(&self, cockpit_name: String, panel: Panel<L>);
}

/// Transmits `Observation`s to the backend
///
/// This struct does **not** implement the `Sync` trait
/// and can therefore not be shared between threads.
/// See `synced()` method.
#[derive(Clone)]
pub struct TelemetryTransmitter<L> {
    sender: mpsc::Sender<TelemetryMessage<L>>,
}

impl<L> TelemetryTransmitter<L>
where
    L: Send + 'static,
{
    /// Get a `TelemetryTransmitterSync`.
    pub fn synced(&self) -> TelemetryTransmitterSync<L> {
        TelemetryTransmitterSync {
            sender: Arc::new(Mutex::new(self.sender.clone())),
        }
    }
}

impl<L> TransmitsTelemetryData<L> for TelemetryTransmitter<L> {
    fn transmit(&self, observation: Observation<L>) {
        if let Err(_err) = self.sender.send(TelemetryMessage::Observation(observation)) {
            // maybe log...
        }
    }

    fn add_handler(&self, handler: Box<HandlesObservations<Label = L>>) {
        if let Err(_err) = self.sender.send(TelemetryMessage::AddHandler(handler)) {
            // maybe log...
        }
    }

    fn add_cockpit(&self, cockpit: Cockpit<L>) {
        if let Err(_err) = self.sender.send(TelemetryMessage::AddCockpit(cockpit)) {
            // maybe log...
        }
    }

    fn add_panel_to_cockpit(&self, cockpit_name: String, panel: Panel<L>) {
        if let Err(_err) = self.sender.send(TelemetryMessage::AddPanel {
            cockpit_name,
            panel,
        }) {
            // maybe log...
        }
    }
}

/// Transmits `Observation`s to the backend and has the `Sync` marker.
///
/// This is almost the same as the `TelemetryTransmitter`.
///
/// Since a `Sender` for a channel is not `Sync` this
/// struct wraps the `Sender` in an `Arc<Mutex<_>>` so that
/// it can be shared between threads.
#[derive(Clone)]
pub struct TelemetryTransmitterSync<L> {
    sender: Arc<Mutex<mpsc::Sender<TelemetryMessage<L>>>>,
}

impl<L> TelemetryTransmitterSync<L>
where
    L: Send + 'static,
{
}

impl<L> TransmitsTelemetryData<L> for TelemetryTransmitterSync<L> {
    fn transmit(&self, observation: Observation<L>) {
        if let Err(_err) = self.sender
            .lock()
            .unwrap()
            .send(TelemetryMessage::Observation(observation))
        {
            // maybe log...
        }
    }

    fn add_handler(&self, handler: Box<HandlesObservations<Label = L>>) {
        if let Err(_err) = self.sender
            .lock()
            .unwrap()
            .send(TelemetryMessage::AddHandler(handler))
        {
            // maybe log...
        }
    }

    fn add_cockpit(&self, cockpit: Cockpit<L>) {
        if let Err(_err) = self.sender
            .lock()
            .unwrap()
            .send(TelemetryMessage::AddCockpit(cockpit))
        {
            // maybe log...
        }
    }

    fn add_panel_to_cockpit(&self, cockpit_name: String, panel: Panel<L>) {
        if let Err(_err) = self.sender
            .lock()
            .unwrap()
            .send(TelemetryMessage::AddPanel {
                cockpit_name,
                panel,
            }) {
            // maybe log...
        }
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
/// the `add_snapshooter` method which is also defined on trait `AggregatesProcessors`.
pub trait PutsSnapshot: Send + 'static {
    /// Puts the current snapshot values into the given `Snapshot` thereby
    /// following the guidelines of `PutsSnapshot`.
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool);
}
