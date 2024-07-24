/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.35.7.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { Coin, StdFee } from "@cosmjs/amino";
import { Addr, Uint128, AssetInfo, InstantiateMsg, RewardConfig, RewardDetail, TimeLockConfig, ExecuteMsg, Binary, CallbackMsg, UpdateConfigMsg, Cw20ReceiveMsg, QueryMsg, Config, ArrayOfTupleOfUint64AndUint128, Boolean, ArrayOfUserRewardByDuration, UserRewardByDuration, UserRewardByLockedAt, UserReward, ArrayOfUserStaking, UserStaking, UserStakingByDuration, ArrayOfStakingWithDuration, StakingWithDuration } from "./.types";
export interface ReadOnlyInterface {
  contractAddress: string;
  config: () => Promise<Config>;
  owner: () => Promise<Addr>;
  totalStaking: () => Promise<Uint128>;
  totalStakingByDuration: () => Promise<ArrayOfStakingWithDuration>;
  staking: ({
    user
  }: {
    user: string;
  }) => Promise<ArrayOfUserStaking>;
  reward: ({
    user
  }: {
    user: string;
  }) => Promise<ArrayOfUserRewardByDuration>;
  calculatePenalty: ({
    amount,
    duration,
    lockedAt
  }: {
    amount: Uint128;
    duration: number;
    lockedAt: number;
  }) => Promise<Uint128>;
  isAllowed: ({
    user
  }: {
    user: string;
  }) => Promise<Boolean>;
  eclipastroRewards: () => Promise<ArrayOfTupleOfUint64AndUint128>;
}
export class QueryClient implements ReadOnlyInterface {
  client: CosmWasmClient;
  contractAddress: string;

  constructor(client: CosmWasmClient, contractAddress: string) {
    this.client = client;
    this.contractAddress = contractAddress;
    this.config = this.config.bind(this);
    this.owner = this.owner.bind(this);
    this.totalStaking = this.totalStaking.bind(this);
    this.totalStakingByDuration = this.totalStakingByDuration.bind(this);
    this.staking = this.staking.bind(this);
    this.reward = this.reward.bind(this);
    this.calculatePenalty = this.calculatePenalty.bind(this);
    this.isAllowed = this.isAllowed.bind(this);
    this.eclipastroRewards = this.eclipastroRewards.bind(this);
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
  totalStaking = async (): Promise<Uint128> => {
    return this.client.queryContractSmart(this.contractAddress, {
      total_staking: {}
    });
  };
  totalStakingByDuration = async (): Promise<ArrayOfStakingWithDuration> => {
    return this.client.queryContractSmart(this.contractAddress, {
      total_staking_by_duration: {}
    });
  };
  staking = async ({
    user
  }: {
    user: string;
  }): Promise<ArrayOfUserStaking> => {
    return this.client.queryContractSmart(this.contractAddress, {
      staking: {
        user
      }
    });
  };
  reward = async ({
    user
  }: {
    user: string;
  }): Promise<ArrayOfUserRewardByDuration> => {
    return this.client.queryContractSmart(this.contractAddress, {
      reward: {
        user
      }
    });
  };
  calculatePenalty = async ({
    amount,
    duration,
    lockedAt
  }: {
    amount: Uint128;
    duration: number;
    lockedAt: number;
  }): Promise<Uint128> => {
    return this.client.queryContractSmart(this.contractAddress, {
      calculate_penalty: {
        amount,
        duration,
        locked_at: lockedAt
      }
    });
  };
  isAllowed = async ({
    user
  }: {
    user: string;
  }): Promise<Boolean> => {
    return this.client.queryContractSmart(this.contractAddress, {
      is_allowed: {
        user
      }
    });
  };
  eclipastroRewards = async (): Promise<ArrayOfTupleOfUint64AndUint128> => {
    return this.client.queryContractSmart(this.contractAddress, {
      eclipastro_rewards: {}
    });
  };
}
export interface Interface extends ReadOnlyInterface {
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
  receive: ({
    amount,
    msg,
    sender
  }: {
    amount: Uint128;
    msg: Binary;
    sender: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  claim: ({
    duration,
    lockedAt
  }: {
    duration: number;
    lockedAt?: number;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  claimAll: ({
    withFlexible
  }: {
    withFlexible: boolean;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  callback: (callbackMsg: CallbackMsg, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  stake: ({
    duration,
    recipient
  }: {
    duration: number;
    recipient?: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  unstake: ({
    amount,
    duration,
    lockedAt,
    recipient
  }: {
    amount?: Uint128;
    duration: number;
    lockedAt?: number;
    recipient?: string;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  restake: ({
    amount,
    fromDuration,
    lockedAt,
    recipient,
    toDuration
  }: {
    amount?: Uint128;
    fromDuration: number;
    lockedAt?: number;
    recipient?: string;
    toDuration: number;
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  allowUsers: ({
    users
  }: {
    users: string[];
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
  blockUsers: ({
    users
  }: {
    users: string[];
  }, fee?: number | StdFee | "auto", memo?: string, _funds?: Coin[]) => Promise<ExecuteResult>;
}
export class Client extends QueryClient implements Interface {
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
    this.receive = this.receive.bind(this);
    this.claim = this.claim.bind(this);
    this.claimAll = this.claimAll.bind(this);
    this.callback = this.callback.bind(this);
    this.stake = this.stake.bind(this);
    this.unstake = this.unstake.bind(this);
    this.restake = this.restake.bind(this);
    this.allowUsers = this.allowUsers.bind(this);
    this.blockUsers = this.blockUsers.bind(this);
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
  receive = async ({
    amount,
    msg,
    sender
  }: {
    amount: Uint128;
    msg: Binary;
    sender: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      receive: {
        amount,
        msg,
        sender
      }
    }, fee, memo, _funds);
  };
  claim = async ({
    duration,
    lockedAt
  }: {
    duration: number;
    lockedAt?: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      claim: {
        duration,
        locked_at: lockedAt
      }
    }, fee, memo, _funds);
  };
  claimAll = async ({
    withFlexible
  }: {
    withFlexible: boolean;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      claim_all: {
        with_flexible: withFlexible
      }
    }, fee, memo, _funds);
  };
  callback = async (callbackMsg: CallbackMsg, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      callback: callbackMsg
    }, fee, memo, _funds);
  };
  stake = async ({
    duration,
    recipient
  }: {
    duration: number;
    recipient?: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      stake: {
        duration,
        recipient
      }
    }, fee, memo, _funds);
  };
  unstake = async ({
    amount,
    duration,
    lockedAt,
    recipient
  }: {
    amount?: Uint128;
    duration: number;
    lockedAt?: number;
    recipient?: string;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      unstake: {
        amount,
        duration,
        locked_at: lockedAt,
        recipient
      }
    }, fee, memo, _funds);
  };
  restake = async ({
    amount,
    fromDuration,
    lockedAt,
    recipient,
    toDuration
  }: {
    amount?: Uint128;
    fromDuration: number;
    lockedAt?: number;
    recipient?: string;
    toDuration: number;
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      restake: {
        amount,
        from_duration: fromDuration,
        locked_at: lockedAt,
        recipient,
        to_duration: toDuration
      }
    }, fee, memo, _funds);
  };
  allowUsers = async ({
    users
  }: {
    users: string[];
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      allow_users: {
        users
      }
    }, fee, memo, _funds);
  };
  blockUsers = async ({
    users
  }: {
    users: string[];
  }, fee: number | StdFee | "auto" = "auto", memo?: string, _funds?: Coin[]): Promise<ExecuteResult> => {
    return await this.client.execute(this.sender, this.contractAddress, {
      block_users: {
        users
      }
    }, fee, memo, _funds);
  };
}