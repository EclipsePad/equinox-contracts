# Equinox Lockdrop

## InstantiateMsg

`owner` is contract owner to update config, `init_timestamp` is lockdrop start timestamp, `deposit_window` is the first part of lockdrop which allows users to deposit and withdraw assets freely, `withdrawal_window` is the second part of lockdrop which allows users to withdraw only, and withdraw only allows for each position only one time and there is withdraw amount limitation with time.
Optional parameters:
`owner` default is instantiator.
`deposit_window` default is 5 days.
`withdrawal_window` default is 2 days.
`lock_configs`

```json
{
  "owner": "neutron...",
  "init_timestamp": 1715698800,
  "deposit_window": 432000,
  "withdrawal_window": 172800,
  "lock_configs": [
    {
      "duration": 0,
      "multiplier": 1,
      "early_unlock_penalty_bps": 5000
    },
    {
      "duration": 2592000,
      "multiplier": 2,
      "early_unlock_penalty_bps": 5000
    },
    {
      "duration": 7776000,
      "multiplier": 6,
      "early_unlock_penalty_bps": 5000
    },
    {
      "duration": 15552000,
      "multiplier": 12,
      "early_unlock_penalty_bps": 5000
    },
    {
      "duration": 23328000,
      "multiplier": 18,
      "early_unlock_penalty_bps": 5000
    },
    {
      "duration": 31536000,
      "multiplier": 24,
      "early_unlock_penalty_bps": 5000
    }
  ],
  "astro_token": "factory/...",
  "xastro_token": "factory/...",
  "eclip": "factory/...",
  "beclip": "neutron...",
  "eclip_staking": "neutron...",
  "astro_staking": "neutron...",
  "blacklist": [],
  "init_early_unlock_penalty": "0.7"
}
```

## ExecuteMsg

### `update_config`

Updates several equinox contracts' addresses for after Equinox is live.

```json
{
  "update_config": {
    "new_config": {
      "single_sided_staking": "neutron...",
      "lp_staking": "neutron...",
      "liquidity_pool": "neutron...",
      "eclipastro_token": "factory/...",
      "voter": "neutron...",
      "eclip_staking": "neutron...",
      "dao_treasury_address": "neutron...",
      "init_early_unlock_penalty": "0.7"
    }
  }
}
```

### `update_reward_distribution_config`

Updates reward vesting config. There is no vesting in default config.

```json
{
  "update_reward_distribution_config": {
    "instant": 10000,
    "vesting_period": 0
  }
}
```

### `update_owner`

Updates contract owner

```json
{
  "update_owner": {
    "new_owner": "neutron..."
  }
}
```

### `increase_lockup`

Deposits ASTRO/xASTRO assets to Lockdrop contract. Only allows on deposit window.

```json
{
  "increase_lockup": {
    "stake_type": "single_staking",
    "duration": 0
  }
}
```

### `extend_lock`

Relocks already locked assets to longer position with optional more deposit assets. Only allows on deposit window and after Equinox is live.

```json
{
  "extend_lock": {
    "stake_type": "lp_staking",
    "from": 0,
    "to": 2592000
  }
}
```

### `unlock`

Unlocks user deposits with optional amounts. During Lockdrop, asset is xASTRO and After Equinox is live, asset is eclipASTRO for single sided staking and eclipASTRO-xASTRO lp token for lp staking.

```json
{
  "unlock": {
    "stake_type": "single_staking",
    "duration": 0,
    "amount": "1000000000"
  }
}
```

### `stake_to_vaults`

Only owner. Owner deposits all the assets of lockdrop contracts to single sided staking vaults and lp staking vaults after Equinox is live. Countdown starts with this function call.

```json
{
  "stake_to_vaults": {}
}
```

### `claim_rewards`

Claims rewards from lockdrop incentives and selected staking vault. User can choose claim assets optionally.

```json
{
  "claim_rewards": {
    "stake_type": "single_staking",
    "duration": 0,
    "assets": [
      {
        "contract_addr": "neutron..."
      },
      {
        "denom": "..."
      }
    ]
  }
}
```

### `claim_all_rewards`

Claims all rewards from all of user positions. User can choose to claim rewards from flexible position. Default is false.

```json
{
  "claim_all_rewards": {
    "stake_type": "single_staking",
    "with_flexible": false,
    "assets": [
      {
        "contract_addr": "neutron..."
      },
      {
        "denom": "..."
      }
    ]
  }
}
```

### `increase_incentives`

Increases lockdrop incentives. Incentive apr is same for each lock duration. Only call this function for depositing ECLIP asset.

```json
{
  "increase_incentives": {
    "rewards":[
      {
        "stake_type":"single_staking",
        "eclip":"1000...",
        "beclip":"1000..."
      },
      {
        "stake_type":"lp_staking",
        "eclip":"1000...",
        "beclip":"1000..."
      }
    ]
  }
}
```

### `claim_blacklist_rewards`

Send blacklist rewards to treasury wallet. Only owner.

```json
{
  "claim_blacklist_rewards": {}
}
```

### `update_lockdrop_periods`

Update deposit/withdrawal periods. Only owner.

```json
{
  "update_lockdrop_periods": {
    "deposit": 123,
    "withdraw": 123
  }
}
```

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### `config`

Returns the lockdrop config.

```json
{
  "config": {}
}
```

### `reward_config`

Returns the lockdrop reward vesting config.

```json
{
  "reward_config": {}
}
```

### `owner`

Returns the lockdrop owner.

```json
{
  "owner": {}
}
```

### `single_lockup_info`

Returns the single sided staking lockup info with pending rewards.

```json
{
  "single_lockup_info": {}
}
```

### `lp_lockup_info`

Returns the lp token staking lockup info with pending rewards.

```json
{
  "lp_lockup_info": {}
}
```

### `single_lockup_state`

Returns the single sided staking lockup state.

```json
{
  "single_lockup_state": {}
}
```

### `lp_lockup_state`

Returns the lp token staking lockup state.

```json
{
  "lp_lockup_state": {}
}
```

### `user_single_lockup_info`

Returns user's single sided staking lockup info.

```json
{
  "user_single_lockup_info": {
    "user": "neutron..."
  }
}
```

### `user_lp_lockup_info`

Returns user's lp token staking lockup info.

```json
{
  "user_lp_lockup_info": {
    "user": "neutron..."
  }
}
```

### `incentives`

Returns ECLIP bECLIP incentives.

```json
{
  "incentives": {
    "stake_type": "single_staking"
  }
}
```

### `blacklist`

Returns blacklist array.

```json
{
  "blacklist": {}
}
```

### `blacklist_rewards`

Returns blacklist rewards.

```json
{
  "blacklist_rewards": {}
}
```

### `calculate_penalty_amount`

Calculates penalty amount.

```json
{
  "calculate_penalty_amount": {
    "amount": "123",
    "duration": 123
  }
}
```
