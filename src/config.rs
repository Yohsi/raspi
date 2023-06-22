use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct Config {
    /// Sensors and series settings 
    pub recorder: Recorder,
    /// HTTP server settings
    pub server: Server,
    /// Path to the sqlite database
    pub db_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct Server {
    /// Port the server should listen to
    pub port: u16,
    /// Host name of the server serving the frontend
    pub allowed_origin: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct Recorder {
    pub sensors: Vec<Sensor>,
    pub series: Vec<SeriesConfig>,
}

#[derive(Deserialize, JsonSchema)]
pub struct Sensor {
    pub id: String,
    pub config: SensorConfig,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SensorConfig {
    Ds18b20(Ds18b20Config),
    Bme280(Bme280Config),
    OpenWeatherMap(OpenWeatherMapConfig),
}

#[derive(Deserialize, JsonSchema)]
pub struct Ds18b20Config {
    /// Serial number of the sensor. Automatically detected if not configured.
    pub serial_number: Option<String>,
    pub temperature_series: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct Bme280Config {
    /// Path to the i2c directory (default "/dev/i2c-1")
    #[serde(default = "Bme280Config::default_path")]
    pub path: String,

    #[serde(default)]
    pub address: Bme280Address,

    pub temperature_series: Option<String>,
    pub humidity_series: Option<String>,
    pub pressure_series: Option<String>,
}

impl Bme280Config {
    fn default_path() -> String {
        return "/dev/i2c-1".to_owned()
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct OpenWeatherMapConfig {
    pub api_key: String,
    pub lat: f64,
    pub lon: f64,
    pub temperature_series: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub enum Bme280Address {
    #[serde(rename = "0x76")]
    SdoGnd,
    #[serde(rename = "0x77")]
    SdoVddio,
}

impl Default for Bme280Address {
    fn default() -> Self {
        Bme280Address::SdoGnd
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct SeriesConfig {
    pub id: String,
    pub name: String,
    pub unit: String,
    pub color: String,
    /// Interval between two measures, in the form "1min", "30sec", "1h", etc.
    pub sampling_interval: String,
}