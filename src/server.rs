use std::error::Error;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time;

use crate::data_store::DataStore;
use crate::timestamp::Timestamp;
use rouille::{content_encoding, try_or_400, Request, Response};

use anyhow::{anyhow, Result};

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

pub fn serve_temperatures(store: Arc<Mutex<DataStore>>, addr: &str) -> Result<()> {
    let serv = rouille::Server::new(addr, move |req| {
        let t1 = time::Instant::now();
        let resp = match (req.method(), req.url().as_str()) {
            ("GET", "/temp") => get_range(req, store.clone()),
            _ => Response::text("No such endpoint").with_status_code(404),
        };
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
        resp.with_additional_header("Access-Control-Allow-Origin", "*")
    })
    .map_err(|e| anyhow!("Error starting server: {e}"))?;
    println!("Listening on {addr}");
    serv.run();
    Ok(())
}

fn get_range(req: &Request, store: Arc<Mutex<DataStore>>) -> Response {
    let from = try_or_400!(req.get_param("from").ok_or(MissingParamErr {
        name: "from".into()
    }));
    let to = try_or_400!(req
        .get_param("to")
        .ok_or(MissingParamErr { name: "to".into() }));

    let from = Timestamp {
        secs: try_or_400!(from.parse()),
    };
    let to = Timestamp {
        secs: try_or_400!(to.parse()),
    };

    let range = store.lock().unwrap().get_range(from, to);

    match range {
        Ok(range) => Response::json(&range),
        Err(err) => Response::with_status_code(
            Response::text(format!("Internal server error: {}", err)),
            500,
        ),
    }
}

