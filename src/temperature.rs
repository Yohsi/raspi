use anyhow::{anyhow, bail, Result};
use reqwest::blocking as reqwest;
use thiserror::Error;
use serde::Deserialize;

#[cfg(target_arch = "arm")]
use anyhow::Context;
#[cfg(target_arch = "arm")]
use std::fs;

#[derive(Error, Debug)]
pub enum DetectSensorError {
    #[error("Cannot list one wire directory content")]
    MissingDirectory(#[from] std::io::Error),

    #[error("Several temperature sensors detected: {}", .0.join(", "))]
    SeveralSensorsDetected(Vec<String>),

    #[error("No temperature sensors detected")]
    NoSensorDetected,
}

#[cfg(target_arch = "arm")]
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

#[cfg(target_arch = "x86_64")]
pub fn detect_ds18b20() -> Result<String, DetectSensorError> {
    // Err(DetectSensorError::SeveralSensorsDetected(vec!["28-041469f3eeff".to_string(), "28-041469f37f5a".to_string()]))
    Ok("28-041469f3eeff".to_owned())
}

#[cfg(target_arch = "arm")]
pub fn read_interior_temperature(sensor_id: &str) -> Result<f32> {
    let ds18b20_file = format!("/sys/bus/w1/devices/{}/temperature", sensor_id);

    let raw_temperature = fs::read_to_string(&ds18b20_file)
        .with_context(|| format!("Failed to read file at {}", &ds18b20_file))?;

    let int_temperature: i32 = raw_temperature.trim().parse().with_context(|| {
        format!(
            "Failed to parse temperature from file content '{}'",
            raw_temperature.escape_debug().to_string()
        )
    })?;

    Ok(int_temperature as f32 / 1000.0)
}

// Stub function to test on a non-raspberry pi computer
#[cfg(target_arch = "x86_64")]
pub fn read_interior_temperature(_: &str) -> Result<f32> {
    Ok(19.3)
}

pub fn fetch_exterior_temperature(lat: f64, lon: f64, api_key: &str) -> Result<f32> {
    let url = format!(
        "https://api.openweathermap.org/data/2.5/weather?lat={lat}&lon={lon}&appid={api_key}&units=metric"
    );
    let resp = reqwest::get(url)?;
    let status = resp.status();
    if !status.is_success() {
        bail!(anyhow!("received an error response: {}", status));
    }
    let json: WeatherApi = resp.json()?;
    Ok(json.main.temp)
}

#[derive(Deserialize, Debug)]
struct WeatherApi {
    main: Measures,
}

#[derive(Deserialize, Debug)]
struct Measures {
    temp: f32,
    // feels_like: f32,
    // temp_min: f32,
    // temp_max: f32,
    // pressure: i32,
    // humidity: i32,
}
