// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The traits for sets of fungible tokens and any associated types.

use super::{
	misc::{AssetId, Balance},
	*,
};
use crate::dispatch::{DispatchError, DispatchResult};
use scale_info::TypeInfo;
use sp_runtime::traits::Saturating;
use sp_std::vec::Vec;

pub mod approvals;
mod balanced;
pub mod enumerable;
pub use enumerable::InspectEnumerable;
pub mod metadata;
pub use balanced::{Balanced, Unbalanced};
mod imbalance;
pub use imbalance::{CreditOf, DebtOf, HandleImbalanceDrop, Imbalance};
pub mod roles;

/// Trait for providing balance-inspection access to a set of named fungible assets.
pub trait Inspect<AccountId> {
	/// Means of identifying one asset class from another.
	type AssetId: AssetId;

	/// Scalar type for representing balance of an account.
	type Balance: Balance;

	/// The total amount of issuance in the system.
	fn total_issuance(asset: Self::AssetId) -> Self::Balance;

	/// The total amount of issuance in the system excluding those which are controlled by the
	/// system.
	fn active_issuance(asset: Self::AssetId) -> Self::Balance {
		Self::total_issuance(asset)
	}

	/// The minimum balance any single account may have.
	fn minimum_balance(asset: Self::AssetId) -> Self::Balance;

	/// Get the `asset` balance of `who`.
	fn balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance;

	/// Get the maximum amount of `asset` that `who` can withdraw/transfer successfully.
	fn reducible_balance(asset: Self::AssetId, who: &AccountId, keep_alive: bool) -> Self::Balance;

	/// Returns `true` if the `asset` balance of `who` may be increased by `amount`.
	///
	/// - `asset`: The asset that should be deposited.
	/// - `who`: The account of which the balance should be increased by `amount`.
	/// - `amount`: How much should the balance be increased?
	/// - `mint`: Will `amount` be minted to deposit it into `account`?
	fn can_deposit(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		mint: bool,
	) -> DepositConsequence;

	/// Returns `Failed` if the `asset` balance of `who` may not be decreased by `amount`, otherwise
	/// the consequence.
	fn can_withdraw(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance>;

	/// Returns `true` if an `asset` exists.
	fn asset_exists(asset: Self::AssetId) -> bool;
}

/// Trait for reading metadata from a fungible asset.
pub trait InspectMetadata<AccountId>: Inspect<AccountId> {
	/// Return the name of an asset.
	fn name(asset: &Self::AssetId) -> Vec<u8>;

	/// Return the symbol of an asset.
	fn symbol(asset: &Self::AssetId) -> Vec<u8>;

	/// Return the decimals of an asset.
	fn decimals(asset: &Self::AssetId) -> u8;
}

/// Trait for providing a set of named fungible assets which can be created and destroyed.
pub trait Mutate<AccountId>: Inspect<AccountId> {
	/// Attempt to increase the `asset` balance of `who` by `amount`.
	///
	/// If not possible then don't do anything. Possible reasons for failure include:
	/// - Minimum balance not met.
	/// - Account cannot be created (e.g. because there is no provider reference and/or the asset
	///   isn't considered worth anything).
	///
	/// Since this is an operation which should be possible to take alone, if successful it will
	/// increase the overall supply of the underlying token.
	fn mint_into(asset: Self::AssetId, who: &AccountId, amount: Self::Balance) -> DispatchResult;

	/// Attempt to reduce the `asset` balance of `who` by `amount`.
	///
	/// If not possible then don't do anything. Possible reasons for failure include:
	/// - Less funds in the account than `amount`
	/// - Liquidity requirements (locks, reservations) prevent the funds from being removed
	/// - Operation would require destroying the account and it is required to stay alive (e.g.
	///   because it's providing a needed provider reference).
	///
	/// Since this is an operation which should be possible to take alone, if successful it will
	/// reduce the overall supply of the underlying token.
	///
	/// Due to minimum balance requirements, it's possible that the amount withdrawn could be up to
	/// `Self::minimum_balance() - 1` more than the `amount`. The total amount withdrawn is returned
	/// in an `Ok` result. This may be safely ignored if you don't mind the overall supply reducing.
	fn burn_from(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError>;

	/// Attempt to reduce the `asset` balance of `who` by as much as possible up to `amount`, and
	/// possibly slightly more due to minimum_balance requirements. If no decrease is possible then
	/// an `Err` is returned and nothing is changed. If successful, the amount of tokens reduced is
	/// returned.
	///
	/// The default implementation just uses `withdraw` along with `reducible_balance` to ensure
	/// that is doesn't fail.
	fn slash(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		Self::burn_from(asset, who, Self::reducible_balance(asset, who, false).min(amount))
	}

	/// Transfer funds from one account into another. The default implementation uses `mint_into`
	/// and `burn_from` and may generate unwanted events.
	fn teleport(
		asset: Self::AssetId,
		source: &AccountId,
		dest: &AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		let extra = Self::can_withdraw(asset, &source, amount).into_result()?;
		// As we first burn and then mint, we don't need to check if `mint` fits into the supply.
		// If we can withdraw/burn it, we can also mint it again.
		Self::can_deposit(asset, dest, amount.saturating_add(extra), false).into_result()?;
		let actual = Self::burn_from(asset, source, amount)?;
		debug_assert!(
			actual == amount.saturating_add(extra),
			"can_withdraw must agree with withdraw; qed"
		);
		match Self::mint_into(asset, dest, actual) {
			Ok(_) => Ok(actual),
			Err(err) => {
				debug_assert!(false, "can_deposit returned true previously; qed");
				// attempt to return the funds back to source
				let revert = Self::mint_into(asset, source, actual);
				debug_assert!(revert.is_ok(), "withdrew funds previously; qed");
				Err(err)
			},
		}
	}
}

/// Trait for providing a set of named fungible assets which can only be transferred.
pub trait Transfer<AccountId>: Inspect<AccountId> {
	/// Transfer funds from one account into another.
	fn transfer(
		asset: Self::AssetId,
		source: &AccountId,
		dest: &AccountId,
		amount: Self::Balance,
		keep_alive: bool,
	) -> Result<Self::Balance, DispatchError>;

	/// Reduce the active issuance by some amount.
	fn deactivate(_: Self::AssetId, _: Self::Balance) {}

	/// Increase the active issuance by some amount, up to the outstanding amount reduced.
	fn reactivate(_: Self::AssetId, _: Self::Balance) {}
}

/// Trait for inspecting a set of named fungible assets which can be placed on hold.
pub trait InspectHold<AccountId>: Inspect<AccountId> {
	/// An identifier for a hold. Used for disambiguating different holds so that
	/// they can be individually replaced or removed and funds from one hold don't accidentally
	/// become released or slashed for another.
	type Reason: codec::Encode + TypeInfo + 'static;

	/// Amount of funds held in hold.
	fn balance_on_hold(
		reason: &Self::Reason,
		asset: Self::AssetId,
		who: &AccountId,
	) -> Self::Balance;

	/// Check to see if some `amount` of `asset` may be held on the account of `who`.
	fn can_hold(asset: Self::AssetId, who: &AccountId, amount: Self::Balance) -> bool;
}

/// Trait for mutating a set of named fungible assets which can be placed on hold.
pub trait MutateHold<AccountId>: InspectHold<AccountId> + Transfer<AccountId> {
	/// Hold some funds in an account.
	fn hold(
		reason: &Self::Reason,
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Release some funds in an account from being on hold.
	///
	/// If `best_effort` is `true`, then the amount actually released and returned as the inner
	/// value of `Ok` may be smaller than the `amount` passed.
	fn release(
		reason: &Self::Reason,
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<Self::Balance, DispatchError>;

	/// Transfer held funds into a destination account.
	///
	/// If `on_hold` is `true`, then the destination account must already exist and the assets
	/// transferred will still be on hold in the destination account. If not, then the destination
	/// account need not already exist, but must be creatable.
	///
	/// If `best_effort` is `true`, then an amount less than `amount` may be transferred without
	/// error.
	///
	/// If `force` is `true`, then other fund-locking mechanisms may be disregarded. It should be
	/// left as `false` in most circumstances, but when you want the same power as a `slash`, it
	/// may be true.
	///
	/// The actual amount transferred is returned, or `Err` in the case of error and nothing is
	/// changed.
	fn transfer_held(
		reason: &Self::Reason,
		asset: Self::AssetId,
		source: &AccountId,
		dest: &AccountId,
		amount: Self::Balance,
		best_effort: bool,
		on_hold: bool,
		force: bool,
	) -> Result<Self::Balance, DispatchError>;
}

/// Trait for mutating one of several types of fungible assets which can be held.
pub trait BalancedHold<AccountId>: Balanced<AccountId> + MutateHold<AccountId> {
	/// Release and slash some funds in an account.
	///
	/// The resulting imbalance is the first item of the tuple returned.
	///
	/// As much funds up to `amount` will be deducted as possible. If this is less than `amount`,
	/// then a non-zero second item will be returned.
	fn slash(
		reason: &Self::Reason,
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> (CreditOf<AccountId, Self>, Self::Balance);
}

/// Trait for providing the ability to create new fungible assets.
pub trait Create<AccountId>: Inspect<AccountId> {
	/// Create a new fungible asset.
	fn create(
		id: Self::AssetId,
		admin: AccountId,
		is_sufficient: bool,
		min_balance: Self::Balance,
	) -> DispatchResult;
}

/// Trait for providing the ability to destroy existing fungible assets.
pub trait Destroy<AccountId>: Inspect<AccountId> {
	/// Start the destruction an existing fungible asset.
	/// * `id`: The `AssetId` to be destroyed. successfully.
	/// * `maybe_check_owner`: An optional account id that can be used to authorize the destroy
	///   command. If not provided, no authorization checks will be performed before destroying
	///   asset.
	fn start_destroy(id: Self::AssetId, maybe_check_owner: Option<AccountId>) -> DispatchResult;

	/// Destroy all accounts associated with a given asset.
	/// `destroy_accounts` should only be called after `start_destroy` has been called, and the
	/// asset is in a `Destroying` state
	///
	/// * `id`: The identifier of the asset to be destroyed. This must identify an existing asset.
	/// * `max_items`: The maximum number of accounts to be destroyed for a given call of the
	///   function. This value should be small enough to allow the operation fit into a logical
	///   block.
	///
	///	Response:
	/// * u32: Total number of approvals which were actually destroyed
	///
	/// Due to weight restrictions, this function may need to be called multiple
	/// times to fully destroy all approvals. It will destroy `max_items` approvals at a
	/// time.
	fn destroy_accounts(id: Self::AssetId, max_items: u32) -> Result<u32, DispatchError>;
	/// Destroy all approvals associated with a given asset up to the `max_items`
	/// `destroy_approvals` should only be called after `start_destroy` has been called, and the
	/// asset is in a `Destroying` state
	///
	/// * `id`: The identifier of the asset to be destroyed. This must identify an existing asset.
	/// * `max_items`: The maximum number of accounts to be destroyed for a given call of the
	///   function. This value should be small enough to allow the operation fit into a logical
	///   block.
	///
	///	Response:
	/// * u32: Total number of approvals which were actually destroyed
	///
	/// Due to weight restrictions, this function may need to be called multiple
	/// times to fully destroy all approvals. It will destroy `max_items` approvals at a
	/// time.
	fn destroy_approvals(id: Self::AssetId, max_items: u32) -> Result<u32, DispatchError>;

	/// Complete destroying asset and unreserve currency.
	/// `finish_destroy` should only be called after `start_destroy` has been called, and the
	/// asset is in a `Destroying` state. All accounts or approvals should be destroyed before
	/// hand.
	///
	/// * `id`: The identifier of the asset to be destroyed. This must identify an existing asset.
	fn finish_destroy(id: Self::AssetId) -> DispatchResult;
}
