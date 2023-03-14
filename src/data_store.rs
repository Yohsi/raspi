use anyhow::{bail, Context, Result};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::timestamp::Timestamp;

#[derive(Serialize)]
pub struct Record {
    time: u64,
    int: Option<f32>,
    ext: Option<f32>,
}

pub struct DataStore {
    current_file: File,
    current_timestamp: Timestamp,
    dir: PathBuf,
    rotation_duration_sec: u64,
}

impl DataStore {
    pub fn new(dir: &Path, rotation_duration_sec: u64) -> Result<DataStore> {
        let now = Timestamp::now();
        let last = Self::existing_files(dir)
            .with_context(|| format!("failed to list csv files in {}", dir.to_string_lossy()))?
            .last()
            .copied();
        let t = match last {
            Some(t) if now - t < rotation_duration_sec => t,
            _ => now,
        };
        let path = dir.join(format!("{t}.csv"));
        let file = File::options()
            .append(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("failed to open file at {}", path.to_string_lossy()))?;
        Ok(DataStore {
            current_file: file,
            current_timestamp: t,
            dir: dir.to_path_buf(),
            rotation_duration_sec,
        })
    }

    pub fn store(&mut self, interior: Option<f32>, exterior: Option<f32>) -> Result<()> {
        self.rotate_if_needed()
            .with_context(|| "unable to rotate store file")?;

        let temp_to_str = |t| format!("{:.2}", t);
        let int = interior.map(temp_to_str).unwrap_or_default();
        let ext = exterior.map(temp_to_str).unwrap_or_default();

        let line = format!("{};{};{}\n", Timestamp::now(), int, ext);
        self.current_file.write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn get_range(&self, from: Timestamp, to: Timestamp) -> Result<Vec<Record>> {
        if from > to {
            bail!("'from' cannot be greater than 'to'");
        }

        let timestamps = Self::existing_files(&self.dir).with_context(|| {
            format!("failed to list csv files in {}", self.dir.to_string_lossy())
        })?;

        let begin = timestamps.partition_point(|&t| t <= from).saturating_sub(1);
        let end = timestamps.partition_point(|&t| t <= to);

        let mut temps = vec![];
        for t in &timestamps[begin..end] {
            let path = self.dir.join(format!("{t}.csv"));
            let file = File::open(&path)
                .with_context(|| format!("failed to open file {}", path.to_string_lossy()))?;

            for line in BufReader::new(file).lines() {
                let record = Self::parse_record_from_csv_line(&line?)
                    .with_context(|| format!("invalid CSV in file {}", path.to_string_lossy()))?;

                if record.time < from.secs {
                    // record is before the range, skip to next
                    continue;
                }
                if record.time > to.secs {
                    // record is after the range, we're done
                    break;
                }

                temps.push(record);
            }
        }

        Ok(temps)
    }

    fn rotate_if_needed(&mut self) -> Result<()> {
        let now = Timestamp::now();
        if now - self.current_timestamp > self.rotation_duration_sec {
            let path = self.dir.join(format!("{now}.csv"));
            if path.exists() {
                bail!(format!(
                    "the file {} already exists",
                    path.to_string_lossy()
                ));
            }
            self.current_file = File::create(&path)
                .with_context(|| format!("failed to create file at {}", path.to_string_lossy()))?;
            self.current_timestamp = now;
        }
        Ok(())
    }

    fn existing_files(dir: &Path) -> Result<Vec<Timestamp>> {
        let mut files: Vec<_> = fs::read_dir(dir)?
            .filter_map(|r| {
                r.map_or(None, |entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .strip_suffix(".csv")
                        .and_then(|trimmed| trimmed.parse().ok().map(|secs| Timestamp { secs }))
                })
            })
            .collect();
        files.sort_unstable();
        Ok(files)
    }

    fn parse_record_from_csv_line(line: &str) -> Result<Record> {
        let mut split: Vec<_> = line.split(";").collect();

        // For compatibility with CSV generated with older version of Raspitemp, 
        // which did not add the trailing semicolon in case of missing ext temp
        if split.len() == 2 {
            split.push("");
        }

        if split.len() != 3 {
            bail!(format!("expected 2 or 3 columns: got \"{}\"", line))
        }
        let time = Timestamp {
            secs: split[0]
                .parse()
                .with_context(|| format!("invalid timestamp: {}", split[0]))?,
        };

        let int_temp = match split[1] {
            "" => None,
            s => Some(
                s.parse()
                    .with_context(|| format!("invalid interior temperature: \"{}\"", s))?,
            ),
        };
        let ext_temp = match split[2] {
            "" => None,
            s => Some(
                s.parse()
                    .with_context(|| format!("invalid exterior temperature: \"{}\"", s))?,
            ),
        };

        Ok(Record {
            time: time.secs,
            int: int_temp,
            ext: ext_temp,
        })
    }
}
