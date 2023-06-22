use crate::config::Ds18b20Config;
use crate::sensors::Sensor;
use anyhow::{anyhow, Context, Result};
use std::fs;
use thiserror::Error;

pub struct Ds18b20 {
    serial_number: String,
    temperature_series: Option<String>,
}

impl Ds18b20 {
    pub fn new(config: Ds18b20Config) -> Result<Ds18b20> {
        let sn = match config.serial_number {
            None => detect_ds18b20()?,
            Some(sn) => sn,
        };
        Ok(Ds18b20 {
            serial_number: sn,
            temperature_series: config.temperature_series,
        })
    }
}

impl Sensor for Ds18b20 {
    fn sample(&mut self, series: &str) -> Result<f64> {
        let ds18b20_file = format!("/sys/bus/w1/devices/{}/temperature", self.serial_number);

        let raw_temperature = fs::read_to_string(&ds18b20_file)
            .with_context(|| format!("Failed to read file at {}", &ds18b20_file))?;

        let int_temperature: i32 = raw_temperature.trim().parse().with_context(|| {
            format!(
                "Failed to parse temperature from file content '{}'",
                raw_temperature.escape_debug().to_string()
            )
        })?;

        if self
            .temperature_series
            .as_ref()
            .is_some_and(|s| s == series)
        {
            Ok(int_temperature as f64 / 1000.0)
        } else {
            Err(anyhow!("no series configured with name {series}"))
        }
    }

    fn series(&self) -> Vec<String> {
        match &self.temperature_series {
            None => vec![],
            Some(s) => vec![s.clone()],
        }
    }
}

#[derive(Error, Debug)]
pub enum DetectSensorError {
    #[error("Cannot list one wire directory content")]
    MissingDirectory(#[from] std::io::Error),

    #[error("Several temperature sensors detected: {}", .0.join(", "))]
    SeveralSensorsDetected(Vec<String>),

    #[error("No temperature sensors detected")]
    NoSensorDetected,
}

pub fn detect_ds18b20() -> Result<String, DetectSensorError> {
    let files: Vec<String> = fs::read_dir("/sys/bus/w1/devices")
        .map_err(|e| DetectSensorError::MissingDirectory(e))?
        .filter_map(|r| match r {
            Err(_) => None,
            Ok(d) => {
                let name = d.file_name().to_string_lossy().to_string();
                if name.starts_with("28-") {
                    Some(name.into())
                } else {
                    None
                }
            }
        })
        .collect();
    match files.len() {
        0 => Err(DetectSensorError::NoSensorDetected),
        1 => Ok(files.into_iter().next().unwrap()),
        _ => Err(DetectSensorError::SeveralSensorsDetected(files)),
    }
}
