pub mod contract;
pub mod entry {
    pub mod execute;
    pub mod instantiate;
    pub mod query;
}
pub mod error;

/// state on the blockchain
pub mod state;
pub use crate::error::ContractError;
