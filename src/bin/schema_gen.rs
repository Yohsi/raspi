use std::fs::File;
use std::io::Write;

use schemars::schema_for;

use raspi::config::Config;

fn main() {
    let schema = schema_for!(Config);
    let mut f = File::create("config.schema.json").expect("Unable to open file config.schema.json");
    let json_schema = serde_json::to_string_pretty(&schema).expect("Unable to generate schema");
    f.write_all(json_schema.as_bytes()).expect("Unable to write in the file");
}