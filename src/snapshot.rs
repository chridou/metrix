use json::{stringify, stringify_pretty, JsonValue};

pub struct JsonConfig {
    /// Serialize `true` as `1` and `false` as `0`
    pub make_booleans_ints: bool,

    /// Configure pretty JSON output.
    ///
    /// Produce pretty JSON with the given indentation if `Some(indentation)`.
    /// If `None` compact JSON is generated.
    pub pretty: Option<u16>,
}

impl Default for JsonConfig {
    fn default() -> JsonConfig {
        JsonConfig {
            make_booleans_ints: false,
            pretty: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetricsSnapshot {
    Group(String, Vec<MetricsSnapshot>),
    GroupWithoutName(Vec<MetricsSnapshot>),
    Panels(Vec<(String, PanelSnapshot)>),
}

impl MetricsSnapshot {
    /// Output JSON with default settings.
    pub fn to_default_json(&self) -> String {
        self.to_json_internal(&JsonConfig::default())
    }

    /// Output JSON with the given settings.
    pub fn to_json(&self, config: &JsonConfig) -> String {
        self.to_json_internal(config)
    }

    fn to_json_internal(&self, config: &JsonConfig) -> String {
        let mut data = JsonValue::new_object();

        self.into_json_value(config, &mut data);

        if let Some(indent) = config.pretty {
            stringify_pretty(data, indent)
        } else {
            stringify(data)
        }
    }

    fn into_json_value(&self, config: &JsonConfig, into: &mut JsonValue) {
        match *self {
            MetricsSnapshot::Panels(ref items) => for &(ref name, ref item) in items {
                into[name] = item.to_json_value(config)
            },
            MetricsSnapshot::Group(ref name, ref items) => {
                let mut grouping = JsonValue::new_object();
                for item in items {
                    item.into_json_value(config, &mut grouping);
                }

                into[name] = grouping
            }
            MetricsSnapshot::GroupWithoutName(ref items) => for item in items {
                item.into_json_value(config, into);
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PanelSnapshot {
    pub counter: Option<(String, u64)>,
    pub gauge: Option<(String, Option<u64>)>,
    pub meter: Option<(String, MeterSnapshot)>,
    pub histogram: Option<(String, HistogramSnapshot)>,
}

impl PanelSnapshot {
    fn to_json_value(&self, config: &JsonConfig) -> JsonValue {
        let mut data = JsonValue::new_object();

        if let Some((ref name, counter)) = self.counter {
            data[name] = counter.into();
        }

        if let Some((ref name, Some(ref gauge))) = self.gauge {
            data[name] = (*gauge).into();
        }

        if let Some((ref name, ref meter)) = self.meter {
            data[name] = meter.to_json_value(config);
        }

        if let Some((ref name, ref histogram)) = self.histogram {
            data[name] = histogram.to_json_value(config);
        }

        data
    }
}

#[derive(Debug, Clone)]
pub struct MeterSnapshot {
    pub one_minute: MeterRate,
    pub five_minutes: MeterRate,
    pub fifteen_minutes: MeterRate,
}

impl MeterSnapshot {
    fn to_json_value(&self, config: &JsonConfig) -> JsonValue {
        object! {
            "one_minute"     => self.one_minute.to_json_value(config),
            "five_minutes"   => self.five_minutes.to_json_value(config),
            "fifteen_minutes"=> self.fifteen_minutes.to_json_value(config),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeterRate {
    pub rate: f64,
    pub count: u64,
}

impl MeterRate {
    fn to_json_value(&self, _config: &JsonConfig) -> JsonValue {
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
    fn to_json_value(&self, _config: &JsonConfig) -> JsonValue {
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
