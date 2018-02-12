use json::{stringify, stringify_pretty, JsonValue};

#[derive(Debug, Clone)]
pub enum MetricsSnapshot {
    NamedGroup(String, Vec<MetricsSnapshot>),
    UnnamedGroup(Vec<MetricsSnapshot>),
    Panels(Vec<(String, PanelSnapshot)>),
}

impl MetricsSnapshot {
    pub fn to_json(&self) -> String {
        stringify(self.to_json_value())
    }

    pub fn to_json_pretty(&self, indent: u16) -> String {
        stringify_pretty(self.to_json_value(), indent)
    }

    fn to_json_value(&self) -> JsonValue {
        let mut data = JsonValue::new_object();

        data
    }
}

#[derive(Debug, Clone)]
pub struct PanelSnapshot {
    pub counter: Option<(String, CounterSnapshot)>,
    pub gauge: Option<(String, GaugeSnapshot)>,
    pub meter: Option<(String, MeterSnapshot)>,
    pub histogram: Option<(String, HistogramSnapshot)>,
}

impl PanelSnapshot {
    fn to_json_value(&self) -> JsonValue {
        let mut data = JsonValue::new_object();

        if let Some((ref name, ref counter)) = self.counter {
            data[name] = counter.to_json_value();
        }

        if let Some((ref name, ref gauge)) = self.gauge {
            data[name] = gauge.to_json_value();
        }

        if let Some((ref name, ref meter)) = self.meter {
            data[name] = meter.to_json_value();
        }

        if let Some((ref name, ref histogram)) = self.histogram {
            data[name] = histogram.to_json_value();
        }

        data
    }
}

#[derive(Debug, Clone)]
pub struct CounterSnapshot {
    pub count: u64,
}

impl CounterSnapshot {
    fn to_json_value(&self) -> JsonValue {
        object! {
            "count" => self.count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GaugeSnapshot {
    pub value: u64,
}

impl GaugeSnapshot {
    fn to_json_value(&self) -> JsonValue {
        object! {
            "value" => self.value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeterSnapshot {
    pub one_minute: MeterRate,
    pub five_minutes: MeterRate,
    pub fifteen_minutes: MeterRate,
}

impl MeterSnapshot {
    fn to_json_value(&self) -> JsonValue {
        object! {
            "one_minute"     => self.one_minute.to_json_value(),
            "five_minutes"   => self.five_minutes.to_json_value(),
            "fifteen_minutes"=> self.fifteen_minutes.to_json_value(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeterRate {
    pub rate: f64,
    pub count: u64,
}

impl MeterRate {
    fn to_json_value(&self) -> JsonValue {
        object! {
            "rate"       => self.rate,
            "count"       => self.count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    pub max: i64,
    pub min: i64,
    pub mean: f64,
    pub stddev: f64,
    pub count: u64,
    pub quantiles: Vec<(u16, i64)>,
}

impl HistogramSnapshot {
    fn to_json_value(&self) -> JsonValue {
        let mut quantiles = JsonValue::new_object();

        for &(ref q, ref v) in &self.quantiles {
            quantiles[format!("p{}", q)] = (*v).into();
        }

        object! {
            "max"       => self.max,
            "min"       => self.min,
            "mean"      => self.mean,
            "stddev"    => self.stddev,
            "count"     => self.count,
            "quantiles" => quantiles,
        }
    }
}
