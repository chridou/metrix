extern crate metrix;

use metrix::TelemetryTransmitterSync;
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

    let mut cockpit = Cockpit::new(name);
    cockpit.add_panel(MetricsLabel::Request, requests_panel);

    let (tx, rx) = TelemetryReceiver::new();

    (tx.synced(), rx)
}

fn main() {
    let driver = TelemetryDriver::default();

    let (telemetry_tx, telemetry_rx) = create_metrics("hallo");

    driver.register_receiver("jjj", Box::new(telemetry_rx));
}
