use std::collections::HashMap;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};

use sensors::{sensor_factory, Sensor};
use series::SeriesState;
use store::Store;

use crate::record::Record;

pub mod config;
mod record;
mod sensors;
mod series;
pub mod server;
mod store;

pub struct Recorder {
    store: Store,
    sensors: HashMap<String, Box<dyn Sensor>>,
    series: HashMap<String, SeriesState>,
    sensor_by_series: HashMap<String, String>,
}

impl Recorder {
    pub fn new(cfg: config::Recorder, db_path: &str) -> Result<Recorder> {
        let store = Store::new(db_path)?;
        let mut sensors = HashMap::new();
        let mut series_state = HashMap::new();
        let mut series_def = vec![];
        let mut sensor_by_series = HashMap::new();

        // Create series
        for series_cfg in cfg.series {
            let prev = series_state.insert(series_cfg.id.clone(), SeriesState::new(&series_cfg)?);
            if let Some(SeriesState { id, .. }) = prev {
                bail!(anyhow!("the \"{id}\" series is defined twice"));
            }
            series_def.push(series_cfg.to_series_def());
        }

        store.update_series(&series_def)?;

        // Create sensors
        for sensor_cfg in cfg.sensors {
            let sensor_id = sensor_cfg.id.clone();
            let sensor = sensor_factory(sensor_cfg.config)?;
            for s in sensor.series() {
                if !series_state.contains_key(&s) {
                    bail!(anyhow!(
                        "the \"{s}\" series associated to \"{sensor_id}\" sensor does not exist"
                    ));
                }
                let prev = sensor_by_series.insert(s.clone(), sensor_id.clone());
                if let Some(prev_sensor) = prev {
                    bail!(anyhow!("the \"{s}\" series is associated to \"{prev_sensor}\" and \"{sensor_id}\" sensors"));
                }
            }
            sensors.insert(sensor_id, sensor);
        }

        // Check for unused series
        for s in series_state.keys() {
            if !sensor_by_series.contains_key(s) {
                bail!(anyhow!(
                    "the \"{s}\" series is not associated to any sensors"
                ));
            }
        }

        Ok(Recorder {
            sensors,
            series: series_state,
            sensor_by_series,
            store,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let next_measure = self.next_measure_instant();
            if let Some(t) = next_measure {
                let now = Instant::now();
                let wait = t.saturating_duration_since(now);
                sleep(wait);
                self.measure()?;
            } else {
                println!("Warning: no series have been configured");
                return Ok(());
            }
        }
    }

    fn next_measure_instant(&self) -> Option<Instant> {
        let mut min = None;
        for (_, s) in self.series.iter() {
            let next = s.next_measure_instant();
            if min.is_none() || next < min.unwrap() {
                min = Some(next);
            }
        }
        min
    }

    fn measure(&mut self) -> Result<()> {
        let now = Instant::now() + Duration::from_secs(1);
        for (id, s) in self.series.iter_mut() {
            let next = s.next_measure_instant();
            if next <= now {
                let sensor_id = &self.sensor_by_series[id];
                let record = self
                    .sensors
                    .get_mut(sensor_id)
                    .ok_or_else(|| anyhow!("no sensor with id {sensor_id}"))?
                    .sample(id)
                    .map(|v| Record {
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        value: v,
                    });

                match &record {
                    Ok(record) => {
                        println!(
                            "Measured \"{}\" series with \"{}\" sensor: got {}",
                            id, sensor_id, record.value
                        );
                        self.store.save(*record, id)?;
                    }
                    Err(e) => {
                        println!("Cannot measure {id} with {sensor_id} sensor: {e}");
                    }
                }
                s.notify_measured(record); // todo: better error handling?
            }
        }
        Ok(())
    }
}
