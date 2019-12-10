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

#[test]
fn gauge_for_all_labels() {
    let mut gauge_adapter = Gauge::new("").for_all_labels();
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(1, Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(2, Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(3, 10));
    assert_eq!(gauge_adapter.gauge().get(), Some(10));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        4,
        (1, TimeUnit::Milliseconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(1_000));
}

#[test]
fn gauge_for_label_label_exists() {
    let mut gauge_adapter = Gauge::new("").for_label(1);
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(1, Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(1, Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(0, Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(1, 10));
    assert_eq!(gauge_adapter.gauge().get(), Some(10));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        1,
        (1, TimeUnit::Milliseconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(1_000));
}

#[test]
fn gauge_for_label_label_does_not_exist() {
    let mut gauge_adapter = Gauge::new("").for_label(1);
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(0, Increment));
    assert_eq!(gauge_adapter.gauge().get(), None);
}

#[test]
fn gauge_for_labels() {
    let mut gauge_adapter = Gauge::new("").for_labels(vec!["a", "b", "c"]);
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("a", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("xxx", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("c", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("yyy", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("b", 10));
    assert_eq!(gauge_adapter.gauge().get(), Some(10));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        "c",
        (1, TimeUnit::Seconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(1_000_000));
}

#[test]
fn gauge_for_label_deltas_only() {
    let mut gauge_adapter = Gauge::new("").for_label_deltas_only("ok");
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("ok", 5));
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("xxx", Increment));
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("ok", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(-1));

    gauge_adapter.handle_observation(&Observation::observed_now("ok", 200));
    assert_eq!(gauge_adapter.gauge().get(), Some(-1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("yyy", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(-1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("ok", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        "ok",
        (1, TimeUnit::Milliseconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));
}

#[test]
fn gauge_inc_dec_on() {
    let mut gauge_adapter = Gauge::new("").inc_dec_on("up", "down");
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up", 5));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(3));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("down", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("xxx", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_now("down", 5_000));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("yyy", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        "down",
        (1, TimeUnit::Milliseconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));
}

#[test]
fn gauge_inc_dec_on_many() {
    let mut gauge_adapter =
        Gauge::new("").inc_dec_on_many(vec!["up1", "up2"], vec!["down1", "down2"]);
    assert_eq!(gauge_adapter.gauge().get(), None);

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up1", 5));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up2", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("up1", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(3));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("down2", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("xxx", Increment));
    assert_eq!(gauge_adapter.gauge().get(), Some(2));

    gauge_adapter.handle_observation(&Observation::observed_now("down1", 5_000));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now("yyy", Decrement));
    assert_eq!(gauge_adapter.gauge().get(), Some(1));

    gauge_adapter.handle_observation(&Observation::observed_one_value_now(
        "down1",
        (1, TimeUnit::Milliseconds),
    ));
    assert_eq!(gauge_adapter.gauge().get(), Some(0));
}
