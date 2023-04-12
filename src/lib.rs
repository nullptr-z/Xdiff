pub mod cli;
mod config;
mod utils;

pub use config::*;
pub use utils::*;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExtraArgs {
    pub headers: Vec<(String, String)>,
    pub query: Vec<(String, String)>,
    pub body: Vec<(String, String)>,
}

impl ExtraArgs {
    pub fn new() -> Self {
        Self::default()
    }
}
