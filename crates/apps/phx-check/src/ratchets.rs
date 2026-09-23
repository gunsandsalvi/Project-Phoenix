use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Ratchets {
    pub ratchet: Vec<Ratchet>,
}

#[derive(Debug, Deserialize)]
pub struct Ratchet {
    pub counter: String,
    pub value: u64,
    pub direction: Direction,
}

/// Which way a counter may move without its entry being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Down,
    Up,
}

pub fn parse(text: &str) -> Result<Ratchets, String> {
    toml::from_str(text).map_err(|e| format!("does not parse: {e}"))
}

impl Ratchets {
    pub fn find(&self, counter: &str) -> Option<&Ratchet> {
        self.ratchet.iter().find(|r| r.counter == counter)
    }
}

impl Ratchet {
    /// Whether a measured value moved the wrong way from the recorded one.
    pub fn breached_by(&self, measured: u64) -> bool {
        match self.direction {
            Direction::Down => measured > self.value,
            Direction::Up => measured < self.value,
        }
    }
}
