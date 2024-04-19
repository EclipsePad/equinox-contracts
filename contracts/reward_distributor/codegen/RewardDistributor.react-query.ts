/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.35.7.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/

import { UseQueryOptions, useQuery, useMutation, UseMutationOptions } from "@tanstack/react-query";
import { ExecuteResult } from "@cosmjs/cosmwasm-stargate";
import { StdFee, Coin } from "@cosmjs/amino";
import { Uint128, InstantiateMsg, LockingRewardConfig, ExecuteMsg, Addr, UpdateConfigMsg, QueryMsg, Config, ArrayOfTupleOfUint64AndUint128, UserRewardResponse, FlexibleReward, TimelockReward, Decimal256, TotalStakingData, StakingData } from "./RewardDistributor.types";
import { RewardDistributorQueryClient, RewardDistributorClient } from "./RewardDistributor.client";
export const rewardDistributorQueryKeys = {
  contract: ([{
    contract: "rewardDistributor"
  }] as const),
  address: (contractAddress: string | undefined) => ([{ ...rewardDistributorQueryKeys.contract[0],
    address: contractAddress
  }] as const),
  config: (contractAddress: string | undefined, args?: Record<string, unknown>) => ([{ ...rewardDistributorQueryKeys.address(contractAddress)[0],
    method: "config",
    args
  }] as const),
  owner: (contractAddress: string | undefined, args?: Record<string, unknown>) => ([{ ...rewardDistributorQueryKeys.address(contractAddress)[0],
    method: "owner",
    args
  }] as const),
  reward: (contractAddress: string | undefined, args?: Record<string, unknown>) => ([{ ...rewardDistributorQueryKeys.address(contractAddress)[0],
    method: "reward",
    args
  }] as const),
  totalStaking: (contractAddress: string | undefined, args?: Record<string, unknown>) => ([{ ...rewardDistributorQueryKeys.address(contractAddress)[0],
    method: "total_staking",
    args
  }] as const),
  pendingRewards: (contractAddress: string | undefined, args?: Record<string, unknown>) => ([{ ...rewardDistributorQueryKeys.address(contractAddress)[0],
    method: "pending_rewards",
    args
  }] as const)
};
export const rewardDistributorQueries = {
  config: <TData = Config,>({
    client,
    options
  }: RewardDistributorConfigQuery<TData>): UseQueryOptions<Config, Error, TData> => ({
    queryKey: rewardDistributorQueryKeys.config(client?.contractAddress),
    queryFn: () => client ? client.config() : Promise.reject(new Error("Invalid client")),
    ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  }),
  owner: <TData = Addr,>({
    client,
    options
  }: RewardDistributorOwnerQuery<TData>): UseQueryOptions<Addr, Error, TData> => ({
    queryKey: rewardDistributorQueryKeys.owner(client?.contractAddress),
    queryFn: () => client ? client.owner() : Promise.reject(new Error("Invalid client")),
    ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  }),
  reward: <TData = UserRewardResponse,>({
    client,
    args,
    options
  }: RewardDistributorRewardQuery<TData>): UseQueryOptions<UserRewardResponse, Error, TData> => ({
    queryKey: rewardDistributorQueryKeys.reward(client?.contractAddress, args),
    queryFn: () => client ? client.reward({
      user: args.user
    }) : Promise.reject(new Error("Invalid client")),
    ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  }),
  totalStaking: <TData = TotalStakingData,>({
    client,
    options
  }: RewardDistributorTotalStakingQuery<TData>): UseQueryOptions<TotalStakingData, Error, TData> => ({
    queryKey: rewardDistributorQueryKeys.totalStaking(client?.contractAddress),
    queryFn: () => client ? client.totalStaking() : Promise.reject(new Error("Invalid client")),
    ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  }),
  pendingRewards: <TData = ArrayOfTupleOfUint64AndUint128,>({
    client,
    options
  }: RewardDistributorPendingRewardsQuery<TData>): UseQueryOptions<ArrayOfTupleOfUint64AndUint128, Error, TData> => ({
    queryKey: rewardDistributorQueryKeys.pendingRewards(client?.contractAddress),
    queryFn: () => client ? client.pendingRewards() : Promise.reject(new Error("Invalid client")),
    ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  })
};
export interface RewardDistributorReactQuery<TResponse, TData = TResponse> {
  client: RewardDistributorQueryClient | undefined;
  options?: Omit<UseQueryOptions<TResponse, Error, TData>, "'queryKey' | 'queryFn' | 'initialData'"> & {
    initialData?: undefined;
  };
}
export interface RewardDistributorPendingRewardsQuery<TData> extends RewardDistributorReactQuery<ArrayOfTupleOfUint64AndUint128, TData> {}
export function useRewardDistributorPendingRewardsQuery<TData = ArrayOfTupleOfUint64AndUint128>({
  client,
  options
}: RewardDistributorPendingRewardsQuery<TData>) {
  return useQuery<ArrayOfTupleOfUint64AndUint128, Error, TData>(rewardDistributorQueryKeys.pendingRewards(client?.contractAddress), () => client ? client.pendingRewards() : Promise.reject(new Error("Invalid client")), { ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  });
}
export interface RewardDistributorTotalStakingQuery<TData> extends RewardDistributorReactQuery<TotalStakingData, TData> {}
export function useRewardDistributorTotalStakingQuery<TData = TotalStakingData>({
  client,
  options
}: RewardDistributorTotalStakingQuery<TData>) {
  return useQuery<TotalStakingData, Error, TData>(rewardDistributorQueryKeys.totalStaking(client?.contractAddress), () => client ? client.totalStaking() : Promise.reject(new Error("Invalid client")), { ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  });
}
export interface RewardDistributorRewardQuery<TData> extends RewardDistributorReactQuery<UserRewardResponse, TData> {
  args: {
    user: string;
  };
}
export function useRewardDistributorRewardQuery<TData = UserRewardResponse>({
  client,
  args,
  options
}: RewardDistributorRewardQuery<TData>) {
  return useQuery<UserRewardResponse, Error, TData>(rewardDistributorQueryKeys.reward(client?.contractAddress, args), () => client ? client.reward({
    user: args.user
  }) : Promise.reject(new Error("Invalid client")), { ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  });
}
export interface RewardDistributorOwnerQuery<TData> extends RewardDistributorReactQuery<Addr, TData> {}
export function useRewardDistributorOwnerQuery<TData = Addr>({
  client,
  options
}: RewardDistributorOwnerQuery<TData>) {
  return useQuery<Addr, Error, TData>(rewardDistributorQueryKeys.owner(client?.contractAddress), () => client ? client.owner() : Promise.reject(new Error("Invalid client")), { ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  });
}
export interface RewardDistributorConfigQuery<TData> extends RewardDistributorReactQuery<Config, TData> {}
export function useRewardDistributorConfigQuery<TData = Config>({
  client,
  options
}: RewardDistributorConfigQuery<TData>) {
  return useQuery<Config, Error, TData>(rewardDistributorQueryKeys.config(client?.contractAddress), () => client ? client.config() : Promise.reject(new Error("Invalid client")), { ...options,
    enabled: !!client && (options?.enabled != undefined ? options.enabled : true)
  });
}
export interface RewardDistributorRelockMutation {
  client: RewardDistributorClient;
  msg: {
    addingAmount?: Uint128;
    from: Addr;
    fromDuration: number;
    relocking: number[][];
    to: Addr;
    toDuration: number;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorRelockMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorRelockMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorRelockMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.relock(msg, fee, memo, funds), options);
}
export interface RewardDistributorTimelockUnstakeMutation {
  client: RewardDistributorClient;
  msg: {
    duration: number;
    lockedAt: number;
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorTimelockUnstakeMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorTimelockUnstakeMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorTimelockUnstakeMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.timelockUnstake(msg, fee, memo, funds), options);
}
export interface RewardDistributorFlexibleUnstakeMutation {
  client: RewardDistributorClient;
  msg: {
    amount: Uint128;
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorFlexibleUnstakeMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorFlexibleUnstakeMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorFlexibleUnstakeMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.flexibleUnstake(msg, fee, memo, funds), options);
}
export interface RewardDistributorTimelockStakeClaimAllMutation {
  client: RewardDistributorClient;
  msg: {
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorTimelockStakeClaimAllMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorTimelockStakeClaimAllMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorTimelockStakeClaimAllMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.timelockStakeClaimAll(msg, fee, memo, funds), options);
}
export interface RewardDistributorTimelockStakeClaimMutation {
  client: RewardDistributorClient;
  msg: {
    duration: number;
    lockedAt: number;
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorTimelockStakeClaimMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorTimelockStakeClaimMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorTimelockStakeClaimMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.timelockStakeClaim(msg, fee, memo, funds), options);
}
export interface RewardDistributorFlexibleStakeClaimMutation {
  client: RewardDistributorClient;
  msg: {
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorFlexibleStakeClaimMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorFlexibleStakeClaimMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorFlexibleStakeClaimMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.flexibleStakeClaim(msg, fee, memo, funds), options);
}
export interface RewardDistributorTimelockStakeMutation {
  client: RewardDistributorClient;
  msg: {
    amount: Uint128;
    duration: number;
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorTimelockStakeMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorTimelockStakeMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorTimelockStakeMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.timelockStake(msg, fee, memo, funds), options);
}
export interface RewardDistributorFlexibleStakeMutation {
  client: RewardDistributorClient;
  msg: {
    amount: Uint128;
    user: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorFlexibleStakeMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorFlexibleStakeMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorFlexibleStakeMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.flexibleStake(msg, fee, memo, funds), options);
}
export interface RewardDistributorUpdateConfigMutation {
  client: RewardDistributorClient;
  msg: {
    config: UpdateConfigMsg;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorUpdateConfigMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorUpdateConfigMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorUpdateConfigMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.updateConfig(msg, fee, memo, funds), options);
}
export interface RewardDistributorUpdateOwnerMutation {
  client: RewardDistributorClient;
  msg: {
    owner: string;
  };
  args?: {
    fee?: number | StdFee | "auto";
    memo?: string;
    funds?: Coin[];
  };
}
export function useRewardDistributorUpdateOwnerMutation(options?: Omit<UseMutationOptions<ExecuteResult, Error, RewardDistributorUpdateOwnerMutation>, "mutationFn">) {
  return useMutation<ExecuteResult, Error, RewardDistributorUpdateOwnerMutation>(({
    client,
    msg,
    args: {
      fee,
      memo,
      funds
    } = {}
  }) => client.updateOwner(msg, fee, memo, funds), options);
}