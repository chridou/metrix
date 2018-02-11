pub struct CockpitSnapshot {
    pub name: String,
    pub panels: Vec<(String, PanelSnapshot)>,
}

pub struct PanelSnapshot {
    pub counter: Option<CounterSnapshot>,
    pub gauge: Option<GaugeSnapshot>,
    pub meter: Option<MeterSnapshot>,
    pub histogram: Option<HistogramSnapshot>,
}

pub struct CounterSnapshot {
    pub name: String,
}

pub struct GaugeSnapshot {
    pub name: String,
}

pub struct MeterSnapshot {
    pub name: String,
}

pub struct HistogramSnapshot {
    pub name: String,
}
