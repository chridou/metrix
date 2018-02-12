extern crate metrix;

use std::thread;
use std::time::{Duration, Instant};
use std::fmt;

use metrix::*;
use metrix::instruments::*;
use metrix::processor::*;
use metrix::driver::*;

#[derive(Clone, PartialEq, Eq)]
enum FooLabel {
    A,
    B,
}

impl fmt::Display for FooLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FooLabel::A => write!(f, "foo_a"),
            FooLabel::B => write!(f, "foo_b"),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum BarLabel {
    A,
    B,
    C,
}

impl fmt::Display for BarLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BarLabel::A => write!(f, "bar_a"),
            BarLabel::B => write!(f, "bar_b"),
            BarLabel::C => write!(f, "bar_c"),
        }
    }
}

fn create_foo_metrics() -> (TelemetryTransmitterSync<FooLabel>, GroupProcessor) {
    let mut foo_a_panel = Panel::default();
    foo_a_panel.add_counter(Counter::new_with_defaults("foo_a_counter"));
    foo_a_panel.add_gauge(Gauge::new_with_defaults("foo_a_gauge"));
    foo_a_panel.add_meter(Meter::new_with_defaults("foo_a_meter"));
    foo_a_panel.add_histogram(Histogram::new_with_defaults("foo_a_histogram"));

    let mut foo_b_panel = Panel::default();
    foo_b_panel.add_counter(Counter::new_with_defaults("foo_b_counter"));
    foo_b_panel.add_gauge(Gauge::new_with_defaults("foo_b_gauge"));
    foo_b_panel.add_meter(Meter::new_with_defaults("foo_b_meter"));
    foo_b_panel.add_histogram(Histogram::new_with_defaults("foo_b_histogram"));

    let mut cockpit = Cockpit::new_with_name("foo_cockpit", None);
    cockpit.add_panel(FooLabel::A, foo_a_panel);
    cockpit.add_panel(FooLabel::B, foo_b_panel);

    let (tx, mut processor) = TelemetryProcessor::new_pair_with_name("processor_foo");

    processor.add_cockpit(cockpit);

    let mut group_processor = GroupProcessor::default();
    group_processor.add_processor(Box::new(processor));

    (tx.synced(), group_processor)
}

fn create_bar_metrics() -> (TelemetryTransmitterSync<BarLabel>, GroupProcessor) {
    let mut bar_a_panel = Panel::default();
    bar_a_panel.add_counter(Counter::new_with_defaults("bar_a_counter"));
    bar_a_panel.add_gauge(Gauge::new_with_defaults("bar_a_gauge"));
    bar_a_panel.add_meter(Meter::new_with_defaults("bar_a_meter"));
    bar_a_panel.add_histogram(Histogram::new_with_defaults("bar_a_histogram"));

    let mut bar_a_cockpit = Cockpit::new_without_name(Some(ValueScaling::NanosToMicros));
    bar_a_cockpit.add_panel(BarLabel::A, bar_a_panel);

    let mut bar_b_panel = Panel::default();
    bar_b_panel.add_counter(Counter::new_with_defaults("bar_b_counter"));
    bar_b_panel.add_gauge(Gauge::new_with_defaults("bar_b_gauge"));
    bar_b_panel.add_meter(Meter::new_with_defaults("bar_b_meter"));
    bar_b_panel.add_histogram(Histogram::new_with_defaults("bar_b_histogram"));

    let mut bar_b_cockpit = Cockpit::new_with_name("bar_b_cockpit", None);
    bar_b_cockpit.add_panel(BarLabel::B, bar_b_panel);

    let mut bar_c_panel = Panel::default();
    bar_c_panel.add_counter(Counter::new_with_defaults("bar_c_counter"));
    bar_c_panel.add_gauge(Gauge::new_with_defaults("bar_c_gauge"));
    bar_c_panel.add_meter(Meter::new_with_defaults("bar_c_meter"));
    bar_c_panel.add_histogram(Histogram::new_with_defaults("bar_c_histogram"));

    let mut bar_c_cockpit = Cockpit::new_with_name("bar_c_cockpit", None);
    bar_c_cockpit.add_panel(BarLabel::C, bar_c_panel);

    let (tx, mut processor) = TelemetryProcessor::new_pair_without_name();

    processor.add_cockpit(bar_a_cockpit);
    processor.add_cockpit(bar_b_cockpit);
    processor.add_cockpit(bar_c_cockpit);

    let mut group_processor1 = GroupProcessor::default();
    group_processor1.add_processor(Box::new(processor));

    let mut group_processor2 = GroupProcessor::default();
    group_processor2.add_processor(Box::new(group_processor1));
    group_processor2.set_name("group_processor_2");

    (tx.synced(), group_processor2)
}

fn main() {
    let mut driver = TelemetryDriver::default();

    let (foo_transmitter, foo_processor) = create_foo_metrics();
    let (bar_transmitter, bar_processor) = create_bar_metrics();

    driver.add_processor(Box::new(foo_processor));
    driver.add_processor(Box::new(bar_processor));

    let start = Instant::now();

    let handle1 = {
        let foo_transmitter = foo_transmitter.clone();
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for n in 0..1_000_000 {
                foo_transmitter.observed_one_value(FooLabel::A, n, Instant::now());
                bar_transmitter.measure_time(BarLabel::C, start);
            }
        })
    };

    let handle2 = {
        let foo_transmitter = foo_transmitter.clone();
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for n in 0..1_000_000 {
                foo_transmitter.observed_one_value(FooLabel::B, n, Instant::now());
                bar_transmitter.observed_one_value(BarLabel::B, n * n, Instant::now());
            }
        })
    };

    let handle3 = {
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for _ in 0..5_000_000 {
                bar_transmitter.observed(BarLabel::A, 1000, Instant::now());
            }
        })
    };

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("{:?}", start.elapsed());

    thread::sleep(Duration::from_secs(5));

    let snapshot = driver.snapshot();

    println!("{}", snapshot.to_json_pretty(4));
}
