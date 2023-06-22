use crate::config::{Bme280Config, Bme280Address::{SdoGnd, SdoVddio}};
use crate::sensors::Sensor;
use anyhow::{anyhow, Context, Result};
use bme280_multibus::{i2c::{Address, Bme280Bus}, Sample};
use linux_embedded_hal::I2cdev;

const SETTINGS: bme280_multibus::Settings = bme280_multibus::Settings {
    config: bme280_multibus::Config::reset()
        .set_standby_time(bme280_multibus::Standby::Millis1000)
        .set_filter(bme280_multibus::Filter::X16),
    ctrl_meas: bme280_multibus::CtrlMeas::reset()
        .set_osrs_t(bme280_multibus::Oversampling::X8)
        .set_osrs_p(bme280_multibus::Oversampling::X8)
        .set_mode(bme280_multibus::Mode::Normal),
    ctrl_hum: bme280_multibus::Oversampling::X8,
};

pub struct Bme280 {
    config: Bme280Config,
    bme280: bme280_multibus::Bme280<Bme280Bus<I2cdev>>,
}

impl Bme280 {
    pub fn new(config: Bme280Config) -> Result<Bme280> {
        let addr = match config.address {
            SdoGnd => Address::SdoGnd,
            SdoVddio => Address::SdoVddio,
        };
        let i2c = I2cdev::new(&config.path).context("Cannot open I2C bus")?;
        let mut bme280 = bme280_multibus::Bme280::from_i2c(i2c, addr)
            .context("Cannot create BME280 driver")?;
        bme280.settings(&SETTINGS).context("Cannot configure BME280")?;

        Ok(Bme280 { config, bme280 })
    }
}

impl Sensor for Bme280 {
    fn sample(&mut self, series: &str) -> Result<f64> {
        let sample: Sample = self.bme280
            .sample()
            .map_err(|e| anyhow!("Cannot read sample from BME280: {:?}", e))?;
        if self.config.humidity_series.as_ref().is_some_and(|s| s == series) {
            return Ok(sample.humidity as f64)
        } else if self.config.pressure_series.as_ref().is_some_and(|s| s == series) {
            return Ok(sample.pressure as f64)
        } else if self.config.temperature_series.as_ref().is_some_and(|s| s == series) {
            return Ok(sample.temperature as f64)
        } else {
            Err(anyhow!("no series configured with name {series}"))
        }
    }

    fn series(&self) -> Vec<String> {
        let mut ret = vec![];
        if let Some(s) = &self.config.temperature_series {
            ret.push(s.clone());
        }
        if let Some(s) = &self.config.humidity_series {
            ret.push(s.clone());
        }
        if let Some(s) = &self.config.pressure_series {
            ret.push(s.clone());
        }
        ret
    }
}
