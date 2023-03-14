use std::{
    cmp::min,
    env,
    path::Path,
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use std::string::String;

use anyhow::{anyhow, Context, Result};
use clap::Parser;

use data_store::DataStore;
use server::serve_temperatures;

mod data_store;
mod server;
mod temperature;
mod timestamp;

#[derive(Parser, Debug)]
struct Args {
    /// out directory
    #[arg(short, long, default_value_t = String::from("out"), value_name = "DIR", value_hint = clap::ValueHint::DirPath)]
    out_dir: String,

    /// Whether to fetch exterior temperature with OpenWeatherMap API
    #[arg(short = 'a', long = "api", requires_all = ["lat", "lon", "api_key"])]
    fetch_ext_with_api: bool,

    /// interval between two interior temperature measures
    #[arg(short, long, default_value_t = ("2min").parse::< humantime::Duration > ().unwrap(), value_name = "INTERVAL")]
    interior_interval: humantime::Duration,

    /// interval between two exterior temperature fetch
    #[arg(short, long, default_value_t = ("20min").parse::< humantime::Duration > ().unwrap(), value_name = "INTERVAL")]
    exterior_interval: humantime::Duration,

    /// latitude for ext temperature fetching
    #[arg(long)]
    lat: Option<f64>,

    /// longitude for ext temperature fetching
    #[arg(long)]
    lon: Option<f64>,

    /// OpenWeatherMap API key, set to "env" to get it from the environment variable OPEN_WEATHER_MAP_API_KEY
    #[arg(short = 'k', long = "key")]
    api_key: Option<String>,

    /// port the server will listen to
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// serial number of the DS18B20 temperature sensor
    #[arg(long)]
    sensor_sn: Option<String>,
}

struct IntConfig {
    interval: Duration,
    sensor_sn: String,
}

struct ExtConfig {
    interval: Duration,
    lon: f64,
    lat: f64,
    api_key: String,
}

const ROTATION_DURATION_SEC: u64 = 60 * 60 * 24 * 7; // 1 week

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            println!("{:#}", err);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    let api_key = match args.api_key.as_deref() {
        Some("env") => Some(env::var("OPEN_WEATHER_MAP_API_KEY")
            .map_err(|_| anyhow!("OPEN_WEATHER_MAP_API_KEY environment variable is not set"))?),
        Some(k) => Some(k.into()),
        _ => None,
    };

    // println!("Starting with {:#?}", args);

    let sensor_sn = match args.sensor_sn {
        Some(name) => name,
        None => {
            println!("Searching for temperature sensors...");
            let sn = temperature::detect_ds18b20()?;
            println!("Found sensor {sn}");
            sn
        }
    };

    let store = Arc::new(Mutex::new(
        DataStore::new(Path::new(&args.out_dir), ROTATION_DURATION_SEC)
            .context("Error creating the data store")?,
    ));

    let store2 = store.clone();
    let port = args.port;

    let server_thread =
        thread::spawn(move || serve_temperatures(store2, &format!("0.0.0.0:{}", port)));

    // Wait to catch server start errors
    thread::sleep(Duration::from_millis(400));

    if server_thread.is_finished() {
    // An error occurred during server startup
        let result = server_thread.join();
        return Err(match result {
            Err(_) => anyhow!("Unexpected panic during server startup"),
            Ok(inner_res) => match inner_res {
                Err(e) => e,
                Ok(()) => anyhow!("Server unexpectedly stopped without errors"),
            },
        });
    } else {
        let int_config = Some(IntConfig {
            interval: args.interior_interval.into(),
            sensor_sn,
        });

        let ext_config = if args.fetch_ext_with_api {
            Some(ExtConfig {
                interval: args.exterior_interval.into(),
                lat: args.lat.unwrap(),
                lon: args.lon.unwrap(),
                api_key: api_key.unwrap(),
            })
        } else {
            None
        };

        let measure_thread = thread::spawn(move || {
            measure(
                store,
                int_config,
                ext_config,
            )
        });

        server_thread.join().unwrap().unwrap();
        measure_thread.join().unwrap();
    }

    Ok(())
}

fn measure(store: Arc<Mutex<DataStore>>, int_config: Option<IntConfig>, ext_config: Option<ExtConfig>) {
    let mut last_ext_fetch = None;
    let mut last_int_measure = None;

    let wait_time = match (&int_config, &ext_config) {
        (None, None) => return,
        (Some(i), None) => i.interval,
        (None, Some(e)) => e.interval,
        (Some(i), Some(e)) => min(i.interval, e.interval),
    };

    loop {
        let mut store_int_temp = None;
        let mut store_ext_temp = None;

        if let Some(int_config) = &int_config {
            if last_int_measure.is_none() || Instant::now() - last_int_measure.unwrap() > int_config.interval {
                match temperature::read_interior_temperature(&int_config.sensor_sn) {
                    Ok(int_temp) => {
                        store_int_temp = Some(int_temp);
                        last_int_measure = Some(Instant::now());
                    }
                    Err(e) => println!("Error measuring interior temperature: {e}"),
                }
            }
        }
        if let Some(ext_config) = &ext_config {
            if last_ext_fetch.is_none() || Instant::now() - last_ext_fetch.unwrap() > ext_config.interval {
                match temperature::fetch_exterior_temperature(ext_config.lat, ext_config.lon, &ext_config.api_key) {
                    Ok(ext_temp) => {
                        store_ext_temp = Some(ext_temp);
                        last_ext_fetch = Some(Instant::now())
                    }
                    Err(e) => println!("Error fetching exterior temperature: {e}"),
                }
            }
        }

        let store_res = store.lock().unwrap().store(store_int_temp, store_ext_temp);
        if let Err(e) = store_res {
            println!("Error storing temperatures: {e}");
        }

        thread::sleep(wait_time);
    }
}
