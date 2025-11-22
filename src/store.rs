use anyhow::Result;
use rusqlite::{params, Connection};

use crate::record::Record;
use crate::series::SeriesDef;

pub struct Store {
    db: Connection,
}

impl Store {
    pub fn new(db_path: &str) -> Result<Store> {
        let db = Connection::open(db_path)?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS records (
            timestamp INT NOT NULL,
            series    TEXT NOT NULL,
            value     REAL NOT NULL,
            PRIMARY KEY (timestamp, series)
        )",
            (),
        )?;
        db.execute(
            "CREATE TABLE IF NOT EXISTS series (
            id        TEXT NOT NULL,
            name      TEXT NOT NULL,
            category  TEXT NOT NULL,
            unit      TEXT NOT NULL,
            color     TEXT NOT NULL,
            PRIMARY KEY (id)
        )",
            (),
        )?;
        Ok(Store { db })
    }

    pub fn save(&self, record: Record, series: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO records (timestamp, series, value) VALUES (?1, ?2, ?3)",
            params![record.timestamp, series, record.value],
        )?;
        Ok(())
    }

    pub fn fetch(&self, series: &str, from: u64, to: u64) -> Result<Vec<Record>> {
        let mut stmt = self.db.prepare(
            "SELECT timestamp, value
             FROM records
             WHERE series = ?1 AND timestamp >= ?2 AND timestamp <= ?3",
        )?;
        let iter = stmt.query_map(params![series, from, to], |row| {
            Ok(Record {
                timestamp: row.get(0)?,
                value: row.get(1)?,
            })
        })?;

        let mut records = vec![];
        for record in iter {
            match record {
                Ok(record) => records.push(record),
                Err(e) => println!("Warning: a record could not be read in the database: {e}"),
            }
        }
        Ok(records)
    }
    pub fn latest(&self, series: &str) -> Result<Record> {
        let mut stmt = self.db.prepare(
            "SELECT timestamp, value
             FROM records
             WHERE series = ?1
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;
        let record = stmt.query_row([series], |row| {
            Ok(Record {
                timestamp: row.get(0)?,
                value: row.get(1)?,
            })
        })?;

        Ok(record)
    }

    pub fn update_series(&self, series: &[SeriesDef]) -> Result<()> {
        let mut stmt = self.db.prepare(
            "INSERT INTO series (id, name, category, unit, color)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                category=excluded.category,
                unit=excluded.unit,
                color=excluded.color
            ",
        )?;
        for s in series {
            stmt.execute([&s.id, &s.name, &s.category, &s.unit, &s.color])?;
        }
        Ok(())
    }

    pub fn series(&self) -> Result<Vec<SeriesDef>> {
        let mut stmt = self.db.prepare(
            "SELECT id, name, category, unit, color
             FROM series",
        )?;
        let series_iter = stmt.query_map([], |row| {
            Ok(SeriesDef {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                unit: row.get(3)?,
                color: row.get(4)?,
            })
        })?;

        let mut series = vec![];
        for s in series_iter {
            series.push(s?);
        }
        Ok(series)
    }
}
