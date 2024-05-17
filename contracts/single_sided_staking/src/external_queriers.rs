use cosmwasm_std::{Deps, StdResult, Uint128};
use equinox_msg::token_converter::{QueryMsg as ConverterQueryMsg, RewardResponse};


pub fn query_eclipastro_pending_rewards(deps: Deps, converter_contract: String) -> StdResult<Uint128> {
    let rewards: RewardResponse = deps
        .querier
        .query_wasm_smart(converter_contract.clone(), &ConverterQueryMsg::Rewards {  })
        .unwrap();
    Ok(rewards.users_reward.amount)
}

pub struct AstroStaking {
    pub total_shares: Uint128,
    pub total_deposit: Uint128,
}
