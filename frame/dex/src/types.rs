// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
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

use super::*;
use core::{fmt::Display, marker::PhantomData};
use sp_std::{cmp::Ordering, fmt::Formatter};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::fungibles::Inspect;
use scale_info::TypeInfo;

pub(super) type AssetBalanceOf<T> =
	<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
pub(super) type PoolIdOf<T> = (<T as Config>::MultiAssetId, <T as Config>::MultiAssetId);

/// Stores what lp_token a particular pool has.
#[derive(Decode, Encode, Default, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
pub struct PoolInfo<PoolAssetId> {
	/// Liquidity pool asset
	pub lp_token: PoolAssetId,
}

// At the moment when using PartialEq on AssetId, native
// is expected to be loweset.
pub trait MultiAssetIdConverter<MultiAssetId, AssetId> {
	fn get_native() -> MultiAssetId;

	fn try_convert(asset: MultiAssetId) -> Result<AssetId, ()>;

	fn into_multiasset_id(asset: AssetId) -> MultiAssetId;
}

/// An implementation of MultiAssetId that chooses between Native and an asset.
#[derive(Decode, Encode, Default, MaxEncodedLen, TypeInfo, Clone, Copy, Debug)]
pub enum NativeOrAssetId<AssetId>
where
	AssetId: Ord,
{
	/// Native asset. For example, on statemint this would be dot.
	#[default]
	Native,
	Asset(AssetId),
}

impl<AssetId: Ord> Ord for NativeOrAssetId<AssetId> {
	fn cmp(&self, other: &Self) -> Ordering {
		match (self, other) {
			(Self::Native, Self::Native) => Ordering::Equal,
			(Self::Native, Self::Asset(_)) => Ordering::Less,
			(Self::Asset(_), Self::Native) => Ordering::Greater,
			(Self::Asset(id1), Self::Asset(id2)) => <AssetId as Ord>::cmp(id1, id2),
		}
	}
}
impl<AssetId: Ord> PartialOrd for NativeOrAssetId<AssetId> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(<Self as Ord>::cmp(self, other))
	}
}
impl<AssetId: Ord> PartialEq for NativeOrAssetId<AssetId> {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == Ordering::Equal
	}
}
impl<AssetId: Ord> Eq for NativeOrAssetId<AssetId> {}

pub struct NativeOrAssetIdConverter<AssetId> {
	_phantom: PhantomData<AssetId>,
}

impl<AssetId: Ord> MultiAssetIdConverter<NativeOrAssetId<AssetId>, AssetId>
	for NativeOrAssetIdConverter<AssetId>
{
	fn get_native() -> NativeOrAssetId<AssetId> {
		NativeOrAssetId::Native
	}

	fn try_convert(asset: NativeOrAssetId<AssetId>) -> Result<AssetId, ()> {
		match asset {
			NativeOrAssetId::Asset(asset) => Ok(asset),
			NativeOrAssetId::Native => Err(()),
		}
	}

	fn into_multiasset_id(asset: AssetId) -> NativeOrAssetId<AssetId> {
		NativeOrAssetId::Asset(asset)
	}
}

impl<AssetId: Ord> Display for NativeOrAssetId<AssetId> {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), core::fmt::Error> {
		todo!()
	}
}
