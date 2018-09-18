# metrix

[![crates.io](https://img.shields.io/crates/v/metrix.svg)](https://crates.io/crates/metrix)
[![docs.rs](https://docs.rs/metrix/badge.svg)](https://docs.rs/metrix)
[![downloads](https://img.shields.io/crates/d/metrix.svg)](https://crates.io/crates/metrix)
[![Build Status](https://travis-ci.org/chridou/metrix.svg?branch=master)](https://travis-ci.org/chridou/metrix)
[![license-mit](http://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/chridou/metrix/blob/master/LICENSE-MIT)
[![license-apache](http://img.shields.io/badge/license-APACHE-blue.svg)](https://github.com/chridou/metrix/blob/master/LICENSE-APACHE)


Metrics for monitoring applications and alerting.

## Goal

Applications/services can have a lot of metrics and one of the greatest
challenges is organizing them. This is what `metrix` tries to help with.

**Metrix** does not aim for providing exact numbers and aims for
applications monitoring only.

This crate is in a very **early** stage and the API might still change.
There may be backends provided for monitoring solutions in the future
but currently only a snapshot that can be
serialized to JSON is provided.

## How does it work

**Metrix** is based on observations collected while running your
application. These observations will then be sent to a backend where
the actual metrics(counters etc.) are updated. For the metrics configured
a snapshot can be queried.

The primary focus of **metrix** is to organize these metrics. There are
several building blocks available. Most of them can have a name that will
then be part of a path within a snapshot.

### Labels

Labels link observations to panels. Labels can be of any type that
implements `Clone + Eq + Send + 'static`. An `enum` is a good choice for a
label.

### Observations

An abservation is made somewhere within your application. When an
observation is sent to the backend it must have a label attached. This label
is then matched against the label of a panel to determine whether an
observation is handled for updating or not.

### Instruments

Instruments are gauges, meters, etc. An instrument gets updated by an
observation where an update is meaningful. Instruments are grouped by
`Panel`s.

You can find instruments in the module `instruments`.

### Panels

A `Panel` groups instruments under same same label. So each instrument
within a panel will be updated by observations that have the same label as
the panel.

Lets say you defined a label `OutgoingRequests`. If you are interested
in the request rate and the latencies. You would then create a panel with a
label `OutgoingRequests` and add a histogram and a meter.

### Cockpit

A cockpit aggregates multiple `Panel`s. A cockpit can be used to monitor
different tasks/parts of a component or worklflow. A cockpit
is bound to a label type.

An example can be that you have service component that calls an external
HTTP client. You could be interested in successful calls and failed calls
individually. So for both cases you would create a value for your label
and then add two panels to the cockpit.

Cockpits are in the module `cockpit`.

### Processors

The most important processor is the `TelemetryProcessor`. It has
a label type as a type parameter and consist of a `TelemetryTransmitter`
that sends observations to the backend(used within your app)
and the actual `TelemetryProcessor` that forms the backend and
processes observations. The `TelemetryProcessor`
can **own** several cockpits for a label type.

There is also a `ProcessorMount` that is label agnostic and can group
several processors. It can also have a name that will be included in the
snapshot.

The processors can be found the module `processor`.

### Driver

The driver **owns** processors and asks the **owned** processors
to process their messages. You need to add your processors to
a driver to start the machinery. A driver is also a processor
which means it can have a name and it can also be part of another
hierarchy.

Each driver has its own thread for polling its processors
so even when attached to another
hierarchy all processors registered with the driver will only
be driven by that driver.

## Recent changes:

* 0.9.1
    * `TelemetryDriver` now supports a processing strategy
* 0.9.0
    * `TelemetryDriver` has a builder
    * `TelemetryDriver` is immutable
    * Snapshots are calculated in the background thread
    * Snapshots can be queried with as a `Future`
* 0.8.3
    * Use crossbeam channels
* 0.8.1
    * Fixed bug always reporting all meter rate if 1 munite was enabled
* 0.8.0
    * Meters only have 1 minute rate enabled by default
    * Histograms can track inactivity and reset themselves
    * Breaking changes: Moved some traits into other packages

## Contributing

Contributing is welcome. Criticism is also welcome!

## License

Metrix is primarily distributed under the terms of
both the MIT license and the Apache License (Version 2.0).

Copyright (c) 2018 Christian Douven


License: Apache-2.0/MIT
