use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{CanonicalAddr, Deps, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{Bucket, ReadonlyBucket, ReadonlySingleton, Singleton};
use moneymarket::custody::{BAssetInfo, BorrowerResponse};

//BLunaAccruedRewardsResponse the struct that shows the result of accrued_rewards query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BLunaAccruedRewardsResponse {
    pub rewards: Uint128,
}

const KEY_CONFIG: &[u8] = b"config";
const PREFIX_BORROWER: &[u8] = b"borrower";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub collateral_token: CanonicalAddr,
    pub overseer_contract: CanonicalAddr,
    pub market_contract: CanonicalAddr,
    pub reward_contract: CanonicalAddr,
    pub liquidation_contract: CanonicalAddr,
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerInfo {
    pub balance: Uint256,
    pub spendable: Uint256,
}

pub fn store_config(storage: &mut dyn Storage, data: &Config) -> StdResult<()> {
    Singleton::new(storage, KEY_CONFIG).save(data)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    ReadonlySingleton::new(storage, KEY_CONFIG).load()
}

pub fn store_borrower_info(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    borrower_info: &BorrowerInfo,
) -> StdResult<()> {
    let mut borrower_bucket: Bucket<BorrowerInfo> = Bucket::new(storage, PREFIX_BORROWER);
    borrower_bucket.save(borrower.as_slice(), borrower_info)?;

    Ok(())
}

pub fn remove_borrower_info(storage: &mut dyn Storage, borrower: &CanonicalAddr) {
    let mut borrower_bucket: Bucket<BorrowerInfo> = Bucket::new(storage, PREFIX_BORROWER);
    borrower_bucket.remove(borrower.as_slice());
}

pub fn read_borrower_info(storage: &dyn Storage, borrower: &CanonicalAddr) -> BorrowerInfo {
    let borrower_bucket: ReadonlyBucket<BorrowerInfo> =
        ReadonlyBucket::new(storage, PREFIX_BORROWER);
    match borrower_bucket.load(borrower.as_slice()) {
        Ok(v) => v,
        _ => BorrowerInfo {
            balance: Uint256::zero(),
            spendable: Uint256::zero(),
        },
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_borrowers(
    deps: Deps,
    start_after: Option<CanonicalAddr>,
    limit: Option<u32>,
) -> StdResult<Vec<BorrowerResponse>> {
    let position_bucket: ReadonlyBucket<BorrowerInfo> =
        ReadonlyBucket::new(deps.storage, PREFIX_BORROWER);

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);

    position_bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            let borrower: CanonicalAddr = CanonicalAddr::from(k);
            Ok(BorrowerResponse {
                borrower: deps.api.addr_humanize(&borrower)?.to_string(),
                balance: v.balance,
                spendable: v.spendable,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<CanonicalAddr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v = addr.as_slice().to_vec();
        v.push(1);
        v
    })
}

// rewards / collateral
const KEY_GLOBAL_INDEX: &[u8] = b"global_index";
const PREFIX_USER_REWARDS: &[u8] = b"user_reward";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, JsonSchema)]
pub struct UserRewards {
    // whenever the user_index < global_index
    // i can set user_index == global_index, and accumulate the rewards

    // 100 units of collateral
    // user_index == 1
    // global_index == 2

    // rewards += (global_index - user_index) * n_collateral_units
    // user_index = global_index
    pub user_index: Decimal256,
    pub rewards: Uint256,
}

pub fn save_global_index(storage: &mut dyn Storage, data: &Decimal256) -> StdResult<()> {
    Singleton::new(storage, KEY_GLOBAL_INDEX).save(data)
}

pub fn read_global_index(storage: &dyn Storage) -> Decimal256 {
    ReadonlySingleton::new(storage, KEY_GLOBAL_INDEX)
        .load()
        .unwrap_or(Decimal256::zero())
}

pub fn read_user_rewards(storage: &dyn Storage, borrower: &CanonicalAddr) -> UserRewards {
    let user_index_bucket: ReadonlyBucket<UserRewards> =
        ReadonlyBucket::new(storage, PREFIX_USER_REWARDS);
    user_index_bucket
        .load(borrower.as_slice())
        .unwrap_or(UserRewards::default())
}

pub fn save_user_rewards(
    storage: &mut dyn Storage,
    borrower: &CanonicalAddr,
    new_rewards: &UserRewards,
) -> StdResult<()> {
    let mut user_index_bucket: Bucket<UserRewards> = Bucket::new(storage, PREFIX_USER_REWARDS);
    user_index_bucket.save(borrower.as_slice(), new_rewards)
}
