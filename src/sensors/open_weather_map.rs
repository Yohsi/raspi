use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::config::OpenWeatherMapConfig;
use crate::sensors::Sensor;

pub struct OpenWeatherMap {
    config: OpenWeatherMapConfig,
}

impl OpenWeatherMap {
    pub fn new(cfg: OpenWeatherMapConfig) -> OpenWeatherMap {
        OpenWeatherMap { config: cfg }
    }
}

impl Sensor for OpenWeatherMap {
    fn sample(&mut self, series: &str) -> Result<f64> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
            self.config.lat, self.config.lon, self.config.api_key
        );
        let resp = reqwest::blocking::get(url)?;
        let status = resp.status();
        if !status.is_success() {
            bail!(anyhow!("received an error response: {}", status));
        }
        let json: WeatherApi = resp.json()?;
        if self
            .config
            .temperature_series
            .as_ref()
            .is_some_and(|s| s == series)
        {
            Ok(json.main.temp)
        } else {
            Err(anyhow!("no series configured with name {series}"))
        }
    }

    fn series(&self) -> Vec<String> {
        match &self.config.temperature_series {
            None => {
                vec![]
            }
            Some(name) => {
                vec![name.clone()]
            }
        }
    }
}

#[derive(Deserialize, Debug)]
struct WeatherApi {
    main: Measures,
}

#[derive(Deserialize, Debug)]
struct Measures {
    temp: f64,
    // feels_like: f64,
    // temp_min: f64,
    // temp_max: f64,
    // pressure: f64,
    // humidity: f64,
}
