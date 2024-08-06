# Single sided staking vault

## InstantiateMsg

`owner` is contract owner for update config, `token` is eclipASTRO token address, 3 types of rewards, eclipASTRO converted from rewards of Astroport ASTRO staking, ECLIP + bECLIP rewards its own.

```json
{
  "owner": "neutron...",
  "token": "neutron...",
  "eclip": "factory...",
  "beclip": "neutron...",
  "timelock_config": [
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
  "token_converter": "neutron...",
  "treasury": "neutron..."
}
```

## ExecuteMsg

### `update_config`

Updates contract config

```json
{
  "update_config": {
    "config": {
      "timelock_config": [
          "..."
      ],
      "token_converter": "neutron...",
      "treasury": "neutron..."
    }
  }
}
```

### `update_reward_config`

Updates contract reward config

```json
{
  "update_reward_config": {
    "details": {
      "eclip": {
        "info": {
          "native_token": {
            "denom": "native..."
          }
        },
        "daily_reward": "123"
      },
      "beclip": {
        "info": {
          "token": {
            "contract_addr": "neutron..."
          }
        },
        "daily_reward": "123"
      }
    },
    "reward_end_time": 123
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

### `claim`

Claims rewards(eclipASTRO, bECLIP, ECLIP) of each duration deposits, optionally only specific deposit.
Users can select assets to claim.

```json
{
  "claim": {
    "duration": 123,
    "locked_at": 123,
    "assets": [
      "..."
    ]
  }
}
```

### `claim_all`

Claims all rewards of user deposits. Has with flexible option.

```json
{
  "claim_all": {
    "with_flexible": false
  }
}
```

### `unstake`

Unstakes user position. Early unstake has penalty. Recipient is optional. For flexible position, remove locked_at or set as 0.

```json
{
  "unstake": {
    "duration": 123,
    "locked_at": 123,
    "amount": "123",
    "recipient": "neutron..."
  }
}
```

### `restake`

Extends user position. Recipient is optional.

```json
{
  "restake": {
    "from_duration": 123,
    "locked_at": 123,
    "to_duration": 123,
    "recipient": "neutron..."
  }
}
```

### `allow_users`

Allows accounts to set relock amount. Normal users can't select amounts to restake.

```json
{
  "allow_users": {
    "users": ["neutron..."]
  }
}
```

### `block_users`

Blocks accounts to set relock amount. Normal users can't select amounts to restake.

```json
{
  "block_users": {
    "users": ["neutron..."]
  }
}
```

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### `config`

Returns the vault config.

```json
{
  "config": {}
}
```

### `reward_config`

Returns the vault reward config.

```json
{
  "reward_config": {}
}
```

### `owner`

Returns the vault owner.

```json
{
  "owner": {}
}
```

### `total_staking`

Returns total amount of eclipASTRO which is staked.

```json
{
  "total_staking": {}
}
```

### `total_staking_by_duration`

Returns total amount of eclipASTRO which is staked for each duration.

```json
{
  "total_staking_by_duration": {}
}
```

### `staking`

Returns user's positions.

```json
{
  "staking": {
    "user": "neutron..."
  }
}
```

### `reward`

Returns user's rewards.

```json
{
  "reward": {
    "user": "neutron..."
  }
}
```

### `calculate_penalty`

Calculates penalty amounts when user unstake early.

```json
{
  "calculate_penalty": {
    "amount": "123",
    "duration": 123,
    "locked_at": 123
  }
}
```

### `is_allowed`

Returns if user is in the allowed list.

```json
{
  "is_allowed": {
    "user": "neutron..."
  }
}
```

### `eclipastro_rewards`

Returns all eclipASTRO rewards which is vesting now.

```json
{
  "total_incentives": {}
}
```
