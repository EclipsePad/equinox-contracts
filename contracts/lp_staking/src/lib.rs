/// Main contract logic
pub mod contract;
/// custom error handler
pub mod error;
/// state on the blockchain
pub mod state;

pub mod config;

pub mod entry {
    pub mod execute;
    pub mod instantiate;
    pub mod migrate;
    pub mod query;
}
