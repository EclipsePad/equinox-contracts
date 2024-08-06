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
  "converter": "neutron...",
  "astroport_incentives": "neutron...",
  "stability_pool": "neutron...",
  "ce_reward_distributor": "neutron...",
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
        "lp_token": "neutron...",
        "lp_contract": "neutron...",
        "converter": "neutron...",
        "astroport_generator": "neutron...",
        "treasury": "neutron...",
        "stability_pool": "neutron...",
        "ce_reward_distributor": "neutron..."
    }
  }
}
```

### `update_reward_config`

Updates contract reward config

```json
{
  "update_reward_config": {
    "distribution": {
      "users": 123,
      "treasury": 123,
      "ce_holders": 123,
      "stability_pool": 123
    },
    "reward_end_time": 123,
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
    }
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
