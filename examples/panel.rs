use std::time::Duration;

use metrix::{instruments::*, snapshot::Snapshot, PutsSnapshot, TimeUnit};

fn main() {
    let data_panel = Panel::named(AcceptAllLabels, "data_freshness")
        .panel(
            Panel::named(
                (
                    Metric::EffectiveDataTimestamp,
                    Metric::EffectiveDataLatency,
                    Metric::DataAgeAlert,
                    Metric::DataAgeWarning,
                ),
                "effective",
            )
            .gauge(Gauge::new_with_defaults("epoch_ms").for_label(Metric::EffectiveDataTimestamp))
            .gauge(
                Gauge::new_with_defaults("latency_ms")
                    .tracking(60)
                    .group_values(true)
                    .display_time_unit(TimeUnit::Milliseconds)
                    .for_label(Metric::EffectiveDataLatency),
            )
            .histogram(
                Histogram::new_with_defaults("latency_distribution_ms")
                    .display_time_unit(TimeUnit::Milliseconds)
                    .for_label(Metric::EffectiveDataLatency),
            )
            .handler(
                StaircaseTimer::new_with_defaults("alert")
                    .switch_off_after(Duration::from_secs(90))
                    .for_label(Metric::DataAgeAlert),
            )
            .handler(
                StaircaseTimer::new_with_defaults("warning")
                    .switch_off_after(Duration::from_secs(90))
                    .for_label(Metric::DataAgeWarning),
            ),
        )
        .panel(
            Panel::named((Metric::DbDataTimestamp, Metric::DbDataLatency), "db")
                .gauge(Gauge::new_with_defaults("epoch_ms").for_label(Metric::DbDataTimestamp))
                .gauge(
                    Gauge::new_with_defaults("latency_ms")
                        .tracking(60)
                        .display_time_unit(TimeUnit::Milliseconds)
                        .group_values(true)
                        .for_label(Metric::DbDataLatency),
                )
                .histogram(
                    Histogram::new_with_defaults("latency_distribution_ms")
                        .display_time_unit(TimeUnit::Milliseconds)
                        .for_label(Metric::DbDataLatency),
                ),
        )
        .panel(
            Panel::named(
                (Metric::CacheDataTimestamp, Metric::CacheDataLatency),
                "cache",
            )
            .gauge(Gauge::new_with_defaults("epoch_ms").for_label(Metric::CacheDataTimestamp))
            .gauge(
                Gauge::new_with_defaults("latency_ms")
                    .tracking(60)
                    .display_time_unit(TimeUnit::Milliseconds)
                    .group_values(true)
                    .for_label(Metric::CacheDataLatency),
            )
            .histogram(
                Histogram::new_with_defaults("latency_distribution_ms")
                    .display_time_unit(TimeUnit::Milliseconds)
                    .for_label(Metric::CacheDataLatency),
            ),
        );

    let mut snapshot = Snapshot::default();
    data_panel.put_snapshot(&mut snapshot, false);
    println!("{}", snapshot.to_default_json());
}

#[derive(Clone, PartialEq, Eq)]
pub enum Metric {
    EffectiveDataTimestamp,
    EffectiveDataLatency,
    DbDataTimestamp,
    DbDataLatency,
    CacheDataTimestamp,
    CacheDataLatency,
    DataAgeAlert,
    DataAgeWarning,
}
