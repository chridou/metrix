#[derive(Debug, Clone, Serialize)]
pub enum TelemetrySnapshot {
    Cockpits(Vec<CockpitSnapshot>),
    Group(Vec<(String, TelemetrySnapshot)>),
}

#[derive(Debug, Clone, Serialize)]
pub struct CockpitSnapshot {
    pub name: String,
    pub panels: Vec<(String, PanelSnapshot)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PanelSnapshot {
    pub counter: Option<CounterSnapshot>,
    pub gauge: Option<GaugeSnapshot>,
    pub meter: Option<MeterSnapshot>,
    pub histogram: Option<HistogramSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CounterSnapshot {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GaugeSnapshot {
    pub name: String,
    pub value: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeterSnapshot {
    pub name: String,
    pub count: u64,
    pub one_minute_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistogramSnapshot {
    pub name: String,
    pub max: i64,
    pub min: i64,
    pub mean: f64,
    pub stddev: f64,
    pub count: u64,
    pub quantiles: Vec<(u16, i64)>,
}
