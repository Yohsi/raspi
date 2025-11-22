use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::SeriesConfig;
use crate::record::Record;

pub struct SeriesState {
    pub id: String,
    pub sampling_interval: Duration,
    pub last_measure_instant: Option<Instant>,
    pub last_measure: Option<Result<Record>>,
}

impl SeriesState {
    pub fn new(cfg: &SeriesConfig) -> Result<Self> {
        let interval = humantime::parse_duration(&cfg.sampling_interval)?;
        Ok(Self {
            id: cfg.id.clone(),
            sampling_interval: interval,
            last_measure_instant: None,
            last_measure: None,
        })
    }

    pub fn next_measure_instant(&self) -> Instant {
        match self.last_measure_instant {
            Some(t) => t + self.sampling_interval,
            None => Instant::now(),
        }
    }

    pub fn notify_measured(&mut self, record: Result<Record>) {
        self.last_measure_instant = Some(Instant::now());
        self.last_measure = Some(record);
    }
}

#[derive(Serialize, Deserialize)]
pub struct SeriesDef {
    pub id: String,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub color: String,
}

impl SeriesDef {
    pub fn new(cfg: SeriesConfig) -> Self {
        Self {
            id: cfg.id,
            name: cfg.name,
            category: cfg.category,
            color: cfg.color,
            unit: cfg.unit,
        }
    }
}
