use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// ASTRO token address
    pub base_token: String,
    /// xASTRO token address
    pub xtoken: String,
    /// vxASTRO contract
    pub vxtoken: String,
    /// contract owner for update
    pub owner: String,
    /// admin for claim rewards
    pub reward_distributor: String,
    /// admin for gauge vote
    pub gauge_voter: String,
    /// Astroport Staking contract
    pub staking_contract: String,
    /// Astroport Gauge contract
    pub gauge_contract: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// stake ASTRO from user
    Receive(Cw20ReceiveMsg),
    /// update config
    UpdateConfig {
        config: Config
    },
    /// update owner
    UpdateOwner {
        owner: String
    },
    /// withdraw xASTRO
    Withdraw {
        amount: Uint128,
    },
    /// withdraw bribe rewards
    WithdrawBribeRewards {},
    /// gauge vote
    Vote {
        // to do
    }
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum QueryMsg {
    /// query config
    Config {},
    /// query owner
    Owner {},
    /// query total vxASTRO
    VotingPower {},
}

#[cw_serde]
pub enum Cw20HookMsg {
    Stake {},
}
