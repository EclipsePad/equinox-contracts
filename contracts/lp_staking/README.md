# Lp staking vault

## InstantiateMsg

`owner` is contract owner for update config, `init_timestamp` is lockdrop start timestamp, `deposit_window` is the first part of lockdrop which allows users to deposit and withdraw assets freely, `withdrawal_window` is the second part of lockdrop which allows users to withdraw only, and withdraw only allows for each position only one time and there is withdraw amount limitation with time.

```json
{
  "owner": "neutron...",
  "lp_token": "neutron...",
  "lp_contract": "neutron...",
  "eclip": "native...",
  "beclip": "neutron...",
  "astro": "native...",
  "xastro": "native...",
  "astro_staking": "neutron...",
  "eclip_staking": "neutron...",
  "stability_pool": "neutron...",
  "astroport_incentives": "neutron...",
  "ce_reward_distributor": "neutron...",
  "treasury": "neutron...",
  "blacklist": []
}
```

## ExecuteMsg

### `update_config`

Updates contract config

```json
{
  "update_config": {
    "config": {
        "lp_token": "neutron...",
        "lp_contract": "neutron...",
        "treasury": "neutron...",
        "stability_pool": "neutron...",
        "ce_reward_distributor": "neutron...",
        "astroport_incentives": "neutron...",
        "eclip": "native...",
        "beclip": "neutron..."
    }
  }
}
```

### `update_reward_distribution`

Updates contract reward config

```json
{
  "update_reward_distribution": {
    "distribution": {
      "users": 123,
      "treasury": 123,
      "ce_holders": 123,
      "stability_pool": 123
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

Claims rewards(ASTRO, bECLIP, ECLIP). ASTRO from Astroport lp token staking rewards, bECLIP and ECLIP from its own.
Optionally, user can set assets to claim

```json
{
  "claim": {
    "assets": [
      {
        "native_token": {
          "denom": "native..."
        }
      },
      {
        "token": {
          "contract_addr": "neutron..."
        }
      }
    ]
  }
}
```

### `stake`

Stakes user deposits.

```json
{
  "Stake": {
    "recipient": "neutron..."
  }
}

### `unstake`

Unstakes user deposits.

```json
{
  "unstake": {
    "amount": "123",
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

### `reward_distribution`

Returns the vault reward config.

```json
{
  "reward_distribution": {}
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

Returns total amount of lp token staked.

```json
{
  "total_staking": {}
}
```

### `staking`

Returns user's staking.

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

### `reward_weights`

Returns each asset's reward weights.

```json
{
  "reward_weights": {}
}
```

### `user_reward_weights`

Returns user's reward weights.

```json
{
  "user_reward_weights": {
    "user": "neutron..."
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
