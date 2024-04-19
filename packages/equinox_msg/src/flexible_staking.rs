use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::reward_distributor::FlexibleReward;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner for updating
    pub owner: String,
    /// eclipASTRO token
    pub token: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the owner
    UpdateOwner {
        owner: String,
    },
    /// Change config
    UpdateConfig {
        config: UpdateConfigMsg,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
    /// Claim rewards of user.
    Claim {},
    Unstake {
        amount: Uint128,
        recipient: Option<String>,
    },
    Relock {
        amount: Option<Uint128>,
        duration: u64,
        recipient: Option<String>,
    },
    AllowUsers {
        users: Vec<String>,
    },
    BlockUsers {
        users: Vec<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// query config
    #[returns(Config)]
    Config {},
    /// query owner
    #[returns(Addr)]
    Owner {},
    /// query total_staking
    #[returns(Uint128)]
    TotalStaking {},
    /// query user_staking
    #[returns(Uint128)]
    Staking { user: String },
    /// query pending_rewards
    #[returns(FlexibleReward)]
    Reward { user: String },
    #[returns(bool)]
    IsAllowed { user: String },
}

#[cw_serde]
pub struct MigrateMsg {
    pub update_contract_name: Option<bool>,
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Stake eclipASTRO token
    Stake {},
    Relock {
        duration: u64,
        amount: Option<Uint128>,
        recipient: Option<String>,
    },
}

#[cw_serde]
pub struct UpdateConfigMsg {
    pub token: Option<String>,
    pub reward_contract: Option<String>,
    pub timelock_contract: Option<String>,
}

#[cw_serde]
pub struct Config {
    /// eclipASTRO token
    pub token: Addr,
    /// reward_contract address
    pub reward_contract: Addr,
    pub timelock_contract: Addr,
}
