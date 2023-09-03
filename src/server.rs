use std::error::Error;
use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time;

use anyhow::{anyhow, Result};
use rouille::{content_encoding, Request, Response, router, try_or_400};
use rusqlite::Error::QueryReturnedNoRows;

use crate::config;
use crate::store::Store;

#[derive(Debug)]
struct MissingParamErr {
    name: String,
}

impl Error for MissingParamErr {}

impl Display for MissingParamErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Missing parameter '{}'", self.name)
    }
}

pub fn serve(cfg: config::Server, db_path: &str) -> Result<()> {
    let store = Mutex::new(Store::new(db_path)?);
    let addr = SocketAddr::new(IpAddr::from([0, 0, 0, 0]), cfg.port);
    let serv = rouille::Server::new(addr, move |req| {
        let t1 = time::Instant::now();
        let resp = router!(req,
            (GET) (/series/{series: String}) => {get_range(req, &series, &store)},
            (GET) (/series/{series: String}/latest) => {get_latest(req, &series, &store)},
            (GET) (/series) => {get_series_name(req, &store)},
            (GET) (/series_def) => {get_series(req, &store)},
            _ => Response::text("No such endpoint").with_status_code(404),
        );
        let resp = content_encoding::apply(&req, resp);
        let t2 = time::Instant::now();
        let ms = (t2 - t1).as_micros() as f32 / 1000.0;
        println!(
            "{} {} -> {} ({:.1}ms)",
            req.method(),
            req.raw_url(),
            resp.status_code,
            ms
        );
        resp.with_additional_header("Access-Control-Allow-Origin", cfg.allowed_origin.clone())
    })
        .map_err(|e| anyhow!("Error starting server: {e}"))?;
    println!("Listening on {addr}");
    serv.run();
    Ok(())
}

fn get_range(req: &Request, series: &str, store: &Mutex<Store>) -> Response {
    let from = try_or_400!(req.get_param("from").ok_or(MissingParamErr {
        name: "from".into()
    }));
    let to = try_or_400!(req
        .get_param("to")
        .ok_or(MissingParamErr { name: "to".into() }));

    let from: u64 = try_or_400!(from.parse());
    let to: u64 = try_or_400!(to.parse());
    let range = store.lock().unwrap().fetch(&series, from, to);

    match range {
        Ok(range) => Response::json(&range),
        Err(err) => Response::with_status_code(
            Response::text(format!("Internal server error: {}", err)),
            500,
        ),
    }
}

fn get_latest(_req: &Request, series: &str, store: &Mutex<Store>) -> Response {
    let latest = store.lock().unwrap().latest(series);

    match latest {
        Ok(latest) => Response::json(&latest),
        Err(err) if err.downcast_ref::<rusqlite::Error>().is_some_and(|e| *e == QueryReturnedNoRows) => Response::with_status_code(
            Response::text(format!("No value found for this series")),
            404,
        ),
        Err(err) => Response::with_status_code(
            Response::text(format!("Internal server error: {}", err)),
            500,
        ),
    }
}

fn get_series_name(_req: &Request, store: &Mutex<Store>) -> Response {
    let series = store.lock().unwrap().series();

    match series {
        Ok(series) => Response::json(&series.iter().map(|s| s.id.clone()).collect::<Vec<_>>()),
        Err(err) => Response::with_status_code(
            Response::text(format!("Internal server error: {}", err)),
            500,
        ),
    }
}

fn get_series(_req: &Request, store: &Mutex<Store>) -> Response {
    let series = store.lock().unwrap().series();

    match series {
        Ok(series) => Response::json(&series),
        Err(err) => Response::with_status_code(
            Response::text(format!("Internal server error: {}", err)),
            500,
        ),
    }
}
