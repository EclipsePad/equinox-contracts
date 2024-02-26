/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.35.7.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { Coin, StdFee } from "@cosmjs/amino";
import { Uint128, InstantiateMsg, LockingRewardConfig, ExecuteMsg, UpdateConfigMsg, QueryMsg, Addr, Config, UserRewardResponse, FlexibleReward, TimelockReward } from "./RewardDistributor.types";
export interface RewardDistributorReadOnlyInterface {
  contractAddress: string;
  config: () => Promise<Config>;
  owner: () => Promise<Addr>;
  reward: ({
    user
  }: {
    user: string;
  }) => Promise<UserRewardResponse>;
}
export class RewardDistributorQueryClient implements RewardDistributorReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.config = this.config.bind(this);
    this.owner = this.owner.bind(this);
    this.reward = this.reward.bind(this);
  }

  config = async (): Promise<Config> => {
    return this.client.queryContractSmart(this.contractAddress, {
      config: {}
    });
  };
  owner = async (): Promise<Addr> => {
    return this.client.queryContractSmart(this.contractAddress, {
      owner: {}
    });
  };
  reward = async ({
    user
  }: {
    user: string;
  }): Promise<UserRewardResponse> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reward: {
        user
      }
    });
  };
}
export interface RewardDistributorInterface extends RewardDistributorReadOnlyInterface {
  contractAddress: string;
  sender: string;
  updateOwner: ({
    owner
  }: {
    owner: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  updateConfig: ({
    config
  }: {
    config: UpdateConfigMsg;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  flexibleStake: ({
    amount,
    user
  }: {
    amount: Uint128;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  timelockStake: ({
    amount,
    duration,
    user
  }: {
    amount: Uint128;
    duration: number;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  flexibleStakeClaim: ({
    user
  }: {
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  timelockStakeClaim: ({
    duration,
    lockedAt,
    user
  }: {
    duration: number;
    lockedAt: number;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  timelockStakeClaimAll: ({
    user
  }: {
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  flexibleUnstake: ({
    amount,
    user
  }: {
    amount: Uint128;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  timelockUnstake: ({
    duration,
    lockedAt,
    user
  }: {
    duration: number;
    lockedAt: number;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  restake: ({
    from,
    lockedAt,
    to,
    user
  }: {
    from: number;
    lockedAt: number;
    to: number;
    user: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
}
export class RewardDistributorClient extends RewardDistributorQueryClient implements RewardDistributorInterface {
  client: SigningCosmWasmClient;
  sender: string;
  contractAddress: string;

  constructor(client: SigningCosmWasmClient, sender: string, contractAddress: string) {
    super(client, contractAddress);
    this.client = client;
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.updateOwner = this.updateOwner.bind(this);
    this.updateConfig = this.updateConfig.bind(this);
    this.flexibleStake = this.flexibleStake.bind(this);
    this.timelockStake = this.timelockStake.bind(this);
    this.flexibleStakeClaim = this.flexibleStakeClaim.bind(this);
    this.timelockStakeClaim = this.timelockStakeClaim.bind(this);
    this.timelockStakeClaimAll = this.timelockStakeClaimAll.bind(this);
    this.flexibleUnstake = this.flexibleUnstake.bind(this);
    this.timelockUnstake = this.timelockUnstake.bind(this);
    this.restake = this.restake.bind(this);
  }

  updateOwner = async ({
    owner
  }: {
    owner: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_owner: {
        owner
      }
    }, fee, memo, _funds);
  };
  updateConfig = async ({
    config
  }: {
    config: UpdateConfigMsg;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      update_config: {
        config
      }
    }, fee, memo, _funds);
  };
  flexibleStake = async ({
    amount,
    user
  }: {
    amount: Uint128;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      flexible_stake: {
        amount,
        user
      }
    }, fee, memo, _funds);
  };
  timelockStake = async ({
    amount,
    duration,
    user
  }: {
    amount: Uint128;
    duration: number;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      timelock_stake: {
        amount,
        duration,
        user
      }
    }, fee, memo, _funds);
  };
  flexibleStakeClaim = async ({
    user
  }: {
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      flexible_stake_claim: {
        user
      }
    }, fee, memo, _funds);
  };
  timelockStakeClaim = async ({
    duration,
    lockedAt,
    user
  }: {
    duration: number;
    lockedAt: number;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      timelock_stake_claim: {
        duration,
        locked_at: lockedAt,
        user
      }
    }, fee, memo, _funds);
  };
  timelockStakeClaimAll = async ({
    user
  }: {
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      timelock_stake_claim_all: {
        user
      }
    }, fee, memo, _funds);
  };
  flexibleUnstake = async ({
    amount,
    user
  }: {
    amount: Uint128;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      flexible_unstake: {
        amount,
        user
      }
    }, fee, memo, _funds);
  };
  timelockUnstake = async ({
    duration,
    lockedAt,
    user
  }: {
    duration: number;
    lockedAt: number;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      timelock_unstake: {
        duration,
        locked_at: lockedAt,
        user
      }
    }, fee, memo, _funds);
  };
  restake = async ({
    from,
    lockedAt,
    to,
    user
  }: {
    from: number;
    lockedAt: number;
    to: number;
    user: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      restake: {
        from,
        locked_at: lockedAt,
        to,
        user
      }
    }, fee, memo, _funds);
  };
}