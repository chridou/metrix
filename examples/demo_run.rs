extern crate metrix;

use std::thread;
use std::time::{Duration, Instant};

use metrix::TelemetryTransmitterSync;
use metrix::TransmitsTelemetryData;
use metrix::telemetry_receiver::ReceivesTelemetryData;
use metrix::telemetry_receiver::{AcceptsSendableReceiver, TelemetryReceiver};
use metrix::instruments::*;
use metrix::driver::*;

use std::fmt;

#[derive(Clone, PartialEq, Eq)]
enum MetricsLabel {
    Request,
}

impl fmt::Display for MetricsLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetricsLabel::Request => write!(f, "request"),
        }
    }
}

fn create_metrics(
    name: &str,
) -> (
    TelemetryTransmitterSync<MetricsLabel>,
    TelemetryReceiver<MetricsLabel>,
) {
    let mut requests_panel = Panel::default();
    requests_panel.add_counter(Counter::new_with_defaults("num_requests"));
    requests_panel.add_gauge(Gauge::new_with_defaults("last_value"));
    requests_panel.add_meter(Meter::new_with_defaults("request_per_second"));
    requests_panel.add_histogram(Histogram::new_with_defaults("the_latency"));

    let mut cockpit = Cockpit::new(name, Some(ValueScaling::NanosToMicros));
    cockpit.add_panel(MetricsLabel::Request, requests_panel);

    let (tx, mut rx) = TelemetryReceiver::new();

    rx.add_cockpit(cockpit);
    (tx.synced(), rx)
}

fn main() {
    let driver = TelemetryDriver::default();

    let (telemetry_tx, telemetry_rx) = create_metrics("hallo");

    driver.register_receiver("demo", Box::new(telemetry_rx));

    let start = Instant::now();

    let handle = thread::spawn(move || {
        for n in 0..1_000_000 {
            telemetry_tx.measure_time(MetricsLabel::Request, start);
        }
    });

    handle.join().unwrap();

    println!("{:?}", start.elapsed());

    thread::sleep(Duration::from_secs(5));

    let snapshot = driver.snapshot();

    println!("{:#?}", snapshot);
}
