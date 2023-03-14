use std::{
    fmt::Display,
    ops::Sub,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    pub secs: u64,
}

impl Timestamp {
    pub fn now() -> Timestamp {
        Timestamp {
            secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl From<u64> for Timestamp {
    fn from(secs: u64) -> Self {
        Timestamp { secs }
    }
}

impl From<Timestamp> for u64 {
    fn from(t: Timestamp) -> Self {
        t.secs
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.secs)
    }
}

impl Sub for Timestamp {
    type Output = u64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.secs - rhs.secs
    }
}

impl Sub<u64> for Timestamp {
    type Output = Timestamp;
    fn sub(self, rhs: u64) -> Self::Output {
        Timestamp {
            secs: self.secs - rhs,
        }
    }
}
