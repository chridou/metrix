use std::time::Instant;

use crate::{Decrement, DecrementBy, Increment, IncrementBy};

use super::*;

#[test]
fn fresh_node_is_empty() {
    let gauge = Gauge::new("");

    assert_eq!(gauge.get(), None);
}

#[test]
fn empty_gauge_updates_by_increment() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(
        Increment.into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(1));
}

#[test]
fn empty_gauge_updates_by_increment_by() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(
        IncrementBy(5).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(5));
}

#[test]
fn empty_gauge_updates_by_decrement() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(
        Decrement.into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(-1));
}

#[test]
fn empty_gauge_updates_by_decrement_by() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(
        DecrementBy(5).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(-5));
}

#[test]
fn empty_gauge_updates_with_value() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(10.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(10));
}

#[test]
fn empty_gauge_updates_with_duration() {
    let mut gauge = Gauge::new("");
    assert_eq!(gauge.get(), None);

    gauge.update(&Update::ObservationWithValue(
        (1, TimeUnit::Milliseconds).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(1_000));
}

#[test]
fn non_empty_gauge_updates_by_increment() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(
        Increment.into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(1));
}

#[test]
fn non_empty_gauge_updates_by_increment_by() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(
        IncrementBy(5).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(5));
}

#[test]
fn non_empty_gauge_updates_by_decrement() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(
        Decrement.into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(-1));
}

#[test]
fn non_empty_gauge_updates_by_decrement_by() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(
        DecrementBy(5).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(-5));
}

#[test]
fn non_empty_gauge_updates_with_value() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(10.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(10));
}

#[test]
fn non_empty_gauge_updates_with_duration() {
    let mut gauge = Gauge::new("");
    gauge.update(&Update::ObservationWithValue(0.into(), Instant::now()));
    assert_eq!(gauge.get(), Some(0));

    gauge.update(&Update::ObservationWithValue(
        (1, TimeUnit::Milliseconds).into(),
        Instant::now(),
    ));
    assert_eq!(gauge.get(), Some(1_000));
}
