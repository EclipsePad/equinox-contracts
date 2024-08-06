use cosmwasm_schema::cw_serde;

use cosmwasm_std::Addr;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub staking_contract: Option<Addr>,
    pub is_registration_enabled: bool,
}

#[cw_serde]
pub struct WhitelistEntry {
    pub wallets: Vec<WalletEntry>,
    pub is_kyc_passed: bool,
    pub user_info: String,
    pub creation_date: u64,
}

#[cw_serde]
pub struct WalletEntry {
    pub network: String,
    pub address: String,
}
