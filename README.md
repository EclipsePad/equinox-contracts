# Eclipse Equinox

## General Contracts
| Name | Description |
|---|---|
| [`Lockdrop`](contracts/lockdrop) | Lockdrop contract for eclipASTRO single sided staking and eclipASTRO-xASTRO lp staking |
| [`single_sided_staking`](contracts/single_sided_staking) | eclipASTRO single sided staking vault |
| [`lp_staking`](contracts/lp_staking) | eclipASTRO-xASTRO lp staking vault |
| [`token_converter`](contracts/token_converter) | Contract that convert ASTRO/xASTRO to eclipASTRO |
| [`token`](contracts/token) | eclipASTRO token |

### You can run tests for all contracts

Run the following from the repository root

```
cargo test
```

### For a production-ready (compressed) build:

Run the following from any contract

```
./build.sh
```

The optimized contracts are generated in the artifacts/ directory.


### You can generate ts code for each contract:

Run the following from any contract

```
./codegen.sh
```
