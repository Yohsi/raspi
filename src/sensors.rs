use anyhow::Result;

use crate::config::SensorConfig;

mod bme280;
mod ds18b20;
mod open_weather_map;

pub trait Sensor {
    fn sample(&mut self, series: &str) -> Result<f64>;
    fn series(&self) -> Vec<String>;
}

pub fn sensor_factory(cfg: SensorConfig) -> Result<Box<dyn Sensor>> {
    match cfg {
        SensorConfig::Ds18b20(cfg) => Ok(Box::new(ds18b20::Ds18b20::new(cfg)?)),
        SensorConfig::Bme280(cfg) => Ok(Box::new(bme280::Bme280::new(cfg)?)),
        SensorConfig::OpenWeatherMap(cfg) => {
            Ok(Box::new(open_weather_map::OpenWeatherMap::new(cfg)))
        }
    }
}
