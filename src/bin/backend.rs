use std::fs::File;
use std::io::BufReader;
use std::thread::sleep;
use std::time::Duration;
use std::{env, thread};

use anyhow::{anyhow, Context, Result};

use raspi::config::Config;
use raspi::server;

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        return Err(anyhow!("missing config file argument"));
    }
    let file = File::open(&args[1])
        .with_context(|| "Cannot open config file")?;
    let reader = BufReader::new(file);

    let cfg: Config = serde_json::from_reader(reader)?;
    let mut recorder = raspi::Recorder::new(cfg.recorder, &cfg.db_path)?;
    let handle = thread::spawn(move || -> Result<()> {
        server::serve(cfg.server, &cfg.db_path)?;
        Ok(())
    });
    sleep(Duration::from_millis(500));
    if handle.is_finished() {
        handle.join().expect("Couldn't join server thread")?;
    }
    recorder.run()?;
    Ok(())
}
