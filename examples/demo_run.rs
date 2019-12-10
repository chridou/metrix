extern crate metrix;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use metrix::cockpit::*;
use metrix::driver::*;
use metrix::instruments::polled::*;
use metrix::instruments::*;
use metrix::processor::*;
use metrix::snapshot::*;
use metrix::*;

#[derive(Clone, PartialEq, Eq)]
enum FooLabel {
    A,
    B,
}

#[derive(Clone, PartialEq, Eq)]
enum BarLabel {
    A,
    B,
    C,
}

struct PolledCounter {
    counter: AtomicUsize,
}

impl PolledCounter {
    pub fn new() -> PolledCounter {
        PolledCounter {
            counter: AtomicUsize::new(0),
        }
    }
}

impl PutsSnapshot for PolledCounter {
    fn put_snapshot(&self, into: &mut Snapshot, _descriptive: bool) {
        let v = self.counter.fetch_add(1, Ordering::SeqCst);
        into.items
            .push(("polled_counter".into(), ItemKind::UInt(v as u64)));
    }
}

fn create_foo_metrics() -> (TelemetryTransmitterSync<FooLabel>, ProcessorMount) {
    let mut foo_a_panel = Panel::with_name(FooLabel::A, "foo_a_panel");
    foo_a_panel.set_counter(Counter::new_with_defaults("foo_a_counter"));
    let mut gauge = Gauge::new_with_defaults("foo_a_gauge");
    gauge.set_title("title");
    gauge.set_description("description");
    foo_a_panel.set_gauge(gauge);
    foo_a_panel.set_meter(Meter::new_with_defaults("foo_a_meter"));
    foo_a_panel.set_histogram(Histogram::new_with_defaults("foo_a_histogram"));
    foo_a_panel.set_title("foo_1_panel_title");
    foo_a_panel.set_description("foo_a_panel_description");

    let mut foo_b_panel = Panel::new(FooLabel::B);
    foo_b_panel.set_counter(Counter::new_with_defaults("foo_b_counter"));
    let mut gauge = Gauge::new_with_defaults("foo_b_gauge");
    gauge.set_title("title");
    gauge.set_description("description");
    foo_b_panel.set_gauge(gauge);
    foo_b_panel.set_meter(Meter::new_with_defaults("foo_b_meter"));
    foo_b_panel.set_histogram(Histogram::new_with_defaults("foo_b_histogram"));
    foo_b_panel.set_title("foo_b_panel_title");
    foo_b_panel.set_description("foo_b_panel_description");

    let polled_counter = PolledCounter::new();
    let mut polled_instrument =
        PollingInstrument::new_with_defaults("polled_instrument_1", polled_counter);
    polled_instrument.set_title("The polled counter 1");
    polled_instrument.set_description("A counter that is increased when a snapshot is polled");
    foo_b_panel.add_snapshooter(polled_instrument);

    let mut cockpit = Cockpit::new("foo_cockpit");
    cockpit.add_panel(foo_a_panel);
    cockpit.add_panel(foo_b_panel);
    cockpit.set_title("foo_cockpit_title");
    cockpit.set_description("foo_cockpit_description");

    let (tx, mut processor) = TelemetryProcessor::new_pair("processor_foo");

    processor.add_cockpit(cockpit);

    let mut group_processor = ProcessorMount::default();
    group_processor.add_processor(processor);

    (tx.synced(), group_processor)
}

fn create_bar_metrics() -> (TelemetryTransmitterSync<BarLabel>, ProcessorMount) {
    let mut bar_a_panel = Panel::named(BarLabel::A, "bar_a_panel");
    bar_a_panel.set_counter(Counter::new_with_defaults("bar_a_counter"));
    bar_a_panel.set_gauge(Gauge::new_with_defaults("bar_a_gauge"));
    bar_a_panel.set_meter(Meter::new_with_defaults("bar_a_meter"));
    bar_a_panel.set_histogram(Histogram::new_with_defaults("bar_a_histogram"));

    let mut bar_a_cockpit = Cockpit::without_name();
    bar_a_cockpit.add_panel(bar_a_panel);

    let mut bar_b_panel = Panel::new(BarLabel::B);
    bar_b_panel.set_counter(Counter::new_with_defaults("bar_b_counter"));
    bar_b_panel.set_gauge(Gauge::new_with_defaults("bar_b_gauge"));
    bar_b_panel.set_meter(Meter::new_with_defaults("bar_b_meter"));
    bar_b_panel.set_histogram(Histogram::new_with_defaults("bar_b_histogram"));

    let mut bar_b_cockpit = Cockpit::new("bar_b_cockpit");
    bar_b_cockpit.add_panel(bar_b_panel);

    let mut bar_c_panel = Panel::named(BarLabel::C, "bar_c_panel");
    bar_c_panel.set_counter(Counter::new_with_defaults("bar_c_counter"));
    bar_c_panel.set_gauge(Gauge::new_with_defaults("bar_c_gauge"));
    bar_c_panel.set_meter(Meter::new_with_defaults("bar_c_meter"));
    bar_c_panel.set_histogram(Histogram::new_with_defaults("bar_c_histogram"));

    let mut bar_c_cockpit = Cockpit::new("bar_c_cockpit");
    bar_c_cockpit.add_panel(bar_c_panel);

    let (tx, mut processor) = TelemetryProcessor::new_pair_without_name();

    processor.add_cockpit(bar_a_cockpit);
    processor.add_cockpit(bar_b_cockpit);
    processor.add_cockpit(bar_c_cockpit);

    let mut group_processor1 = ProcessorMount::default();
    group_processor1.add_processor(processor);

    let mut group_processor2 = ProcessorMount::default();
    group_processor2.add_processor(group_processor1);
    group_processor2.set_name("group_processor_2");

    let polled_counter = PolledCounter::new();
    let mut polled_instrument =
        PollingInstrument::new_with_defaults("polled_instrument_2", polled_counter);
    polled_instrument.set_title("The polled counter 2");
    polled_instrument.set_description("A counter that is increased when a snapshot is polled");

    group_processor2.add_snapshooter(polled_instrument);

    (tx.synced(), group_processor2)
}

fn main() {
    let builder = DriverBuilder::new("demo");
    let mut driver = builder.build();
    //driver.change_processing_stragtegy(ProcessingStrategy::DropAll);
    //driver.pause();

    let (foo_transmitter, foo_processor) = create_foo_metrics();
    let (bar_transmitter, bar_processor) = create_bar_metrics();

    driver.add_processor(foo_processor);
    driver.add_processor(bar_processor);

    let polled_counter = PolledCounter::new();
    let mut polled_instrument =
        PollingInstrument::new_with_defaults("polled_instrument_3", polled_counter);
    polled_instrument.set_title("The polled counter 3");
    polled_instrument.set_description("A counter that is increased when a snapshot is polled");

    driver.add_snapshooter(polled_instrument);

    let start = Instant::now();

    let handle1 = {
        let foo_transmitter = foo_transmitter.clone();
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for n in 0..5_000_000 {
                foo_transmitter.observed_one_value(FooLabel::A, n, Instant::now());
                bar_transmitter.measure_time(BarLabel::C, start);
            }
        })
    };

    // Poll a snapshot for the counter
    let _ = driver.snapshot(true).unwrap();

    let handle2 = {
        let foo_transmitter = foo_transmitter.clone();
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for n in 0..5_000_000 {
                foo_transmitter.observed_one_value(FooLabel::B, n, Instant::now());
                bar_transmitter.observed_one_value(BarLabel::B, n * n, Instant::now());
            }
        })
    };

    // Poll a snapshot for the counter
    let _ = driver.snapshot(true).unwrap();

    let handle3 = {
        let bar_transmitter = bar_transmitter.clone();

        thread::spawn(move || {
            for i in 0..5_000_000 {
                bar_transmitter.observed_one_value(BarLabel::A, i, Instant::now());
            }
        })
    };

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    //driver.resume();

    println!(
        "Sending observations took {:?}. Sleeping 1 secs to collect remaining data. \
         Depending on your machine you might see that not all metrics have a count \
         of 5 million obseravtions.",
        start.elapsed()
    );

    thread::sleep(Duration::from_secs(1));

    println!("\n\n\n=======================\n\n");

    println!(
        "Get snapshot. If it still blocks here there are still many messges to be processed..."
    );

    println!("\n\n\n=======================\n\n");

    let snapshot = driver.snapshot(true).unwrap();

    let mut config = JsonConfig::default();
    config.pretty = Some(4);

    println!("{:?}", snapshot);
    println!("\n\n\n=======================\n\n");
    println!("{}", snapshot.to_json(&config));
}
