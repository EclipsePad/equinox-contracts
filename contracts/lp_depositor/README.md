# eclipASTRO converter

## InstantiateMsg

`owner` is contract owner for update config, `token_code_id` is eclipASTRO token code id.

```json
{
  "owner": "neutron...",
  "astro": "native...",
  "xastro": "native...",
  "staking_contract": "neutron...",
  "treasury": "neutron...",
  "token_code_id": 123,
  "marketing": {
    "project": "...",
    "description": "...",
    "marketing": "...",
    "logo": "..."
  }
}
```

## ExecuteMsg

### `update_config`

Updates contract config

```json
{
  "update_config": {
    "config": {
      "vxastro_holder": "neutron...",
      "treasury": "neutron...",
      "stability_pool": "neutron...",
      "single_staking_contract": "neutron...",
      "ce_reward_distributor": "neutron..."
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

### `update_reward_config`

Updates reward config

```json
{
  "update_reward_config": {
    "config": {
      "users": 123,
      "treasury": 123,
      "ce_holders": 123,
      "stability_pool": 123
    }
  }
}
```

### `claim`

Claims Astroport ASTRO staking rewards and distribute them to single sided staking vault, treasury, etc. Treasury reward doesn't claim automatically until owner calls `claim_treasur_reward`.

```json
{
  "claim": {}
}
```

### `claim_treasury_reward`

Claims treasury reward and sends it to treasury wallet. Only owner can call this function.

```json
{
  "claim_treasury_reward": {
    "amount": 123
  }
}
```

### `withdraw_available_balance`

Withdraws xASTRO which was redistributed as eclipASTRO to single sided vault.

```json
{
  "withdraw_available_balance": {
    "amount": "123",
    "recipient": "neutron..."
  }
}
```

## QueryMsg

All query messages are described below. A custom struct is defined for each query response.

### `config`

Returns the converter config.

```json
{
  "config": {}
}
```

### `owner`

Returns the converter owner.

```json
{
  "owner": {}
}
```

### `reward_config`

Returns the reward config.

```json
{
  "reward_config": {}
}
```

### `rewards`

Returns pending rewards.

```json
{
  "rewards": {}
}
```

### `withdrawable_balance`

Returns withdrawable xASTRO amount.

```json
{
  "withdrawable_balance": {}
}
```

### `stake_info`

Returns total staked ASTRO, xASTRO, claimed xASTRO balance.

```json
{
  "stake_info": {}
}
```
