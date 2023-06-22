use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: u64,
    pub value: f64,
}
