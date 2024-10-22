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
  "eclip_staking": "neutron...",
  "voter": "neutron...",
  "treasury": "neutron...",
  "blacklist": [],
  "init_early_unlock_penalty": "0.1"
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
      "voter": "neutron...",
      "treasury": "neutron...",
      "eclip": "neutron...",
      "beclip": "neutron...",
      "eclip_staking": "neutron...",
      "init_early_unlock_penalty": "0.1"
    }
  }
}
```

### `propose_new_owner`

Updates contract owner

```json
{
  "propose_new_owner": {
    "owner": "neutron...",
    "expires_in": 123
  }
}
```

### `drop_ownership_proposal`

Don't update contract owner

```json
{
  "drop_ownership_proposal": {}
}
```

### `claim_ownership`

Update contract owner

```json
{
  "claim_ownership": {}
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

### `stake`

Stakes eclipASTRO.

```json
{
  "stake": {
    "duration": 123,
    "recipient": "neutron..."
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
    "amount": "123",
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

### `add_rewards`

Add rewards every month.

```json
{
  "add_rewards": {
    "from": 123,
    "duration": 123,
    "eclip": "123",
    "beclip": "123"
  }
}
```

### `claim_blacklist_rewards`

Send blacklist rewards to treasury.

```json
{
  "claim_blacklist_rewards": {}
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
    "user": "neutron...",
    "duration": 123,
    "locked_at": 123
  }
}
```

### `calculate_reward`

Calculates user reward.

```json
{
  "calculate_reward": {
    "amount": "123",
    "duration": 123,
    "locked_at": 123,
    "from": 123,
    "to": 123
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
  "eclipastro_rewards": {}
}
```

### `blacklist`

Returns blacklist.

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

### `reward_schedule`

Returns reward schedule.

```json
{
  "reward_schedule": {
    "from": 123
  }
}
```

### `reward_list`

Returns all rewards of user.

```json
{
  "reward_list": {
    "user": "neutron..."
  }
}
```
