// This file is part of Substrate.

// Copyright (C) 2017-2022 Parity Technologies (UK) Ltd.
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

//! Hashing Functions.

#![warn(missing_docs)]

use crate::utils::serialize_result;
use ark_bls12_381::{g1, g2, Bls12_381, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ec::{
	models::CurveConfig,
	pairing::{MillerLoopOutput, Pairing},
	short_weierstrass::SWCurveConfig,
};
use ark_serialize::{CanonicalDeserialize, Compress, Validate};
use ark_std::io::Cursor;
use sp_std::vec::Vec;

/// Compute multi miller loop through arkworks
pub fn multi_miller_loop(a_vec: Vec<Vec<u8>>, b_vec: Vec<Vec<u8>>) -> Vec<u8> {
	let g1: Vec<_> = a_vec
		.iter()
		.map(|a| {
			let cursor = Cursor::new(a);
			<Bls12_381 as Pairing>::G1Prepared::deserialize_with_mode(
				cursor,
				Compress::Yes,
				Validate::No,
			)
			// .map(<Bls12_381 as Pairing>::G1Prepared::from)
			.unwrap()
		})
		.collect();
	let g2: Vec<_> = b_vec
		.iter()
		.map(|b| {
			let cursor = Cursor::new(b);
			<Bls12_381 as Pairing>::G2Affine::deserialize_with_mode(
				cursor,
				Compress::Yes,
				Validate::No,
			)
			.map(<Bls12_381 as Pairing>::G2Prepared::from)
			.unwrap()
		})
		.collect();

	let result = Bls12_381::multi_miller_loop(g1, g2).0;

	serialize_result(result)
}

/// Compute final exponentiation through arkworks
pub fn final_exponentiation(target: Vec<u8>) -> Vec<u8> {
	let cursor = Cursor::new(target);
	let target = <Bls12_381 as Pairing>::TargetField::deserialize_with_mode(
		cursor,
		Compress::No,
		Validate::No,
	)
	.unwrap();

	let result = Bls12_381::final_exponentiation(MillerLoopOutput(target)).unwrap().0;

	serialize_result(result)
}

/// Compute a scalar multiplication on G2 through arkworks
pub fn mul_projective_g2(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8> {
	let cursor = Cursor::new(base);
	let base = G2Projective::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let cursor = Cursor::new(scalar);
	let scalar = Vec::<u64>::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let result = <g2::Config as SWCurveConfig>::mul_projective(&base, &scalar);

	serialize_result(result)
}

/// Compute a scalar multiplication on G2 through arkworks
pub fn mul_projective_g1(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8> {
	let cursor = Cursor::new(base);
	let base = G1Projective::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let cursor = Cursor::new(scalar);
	let scalar = Vec::<u64>::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let result = <g1::Config as SWCurveConfig>::mul_projective(&base, &scalar);

	serialize_result(result)
}

/// Compute a scalar multiplication on G2 through arkworks
pub fn mul_affine_g1(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8> {
	let cursor = Cursor::new(base);
	let base = G1Affine::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let cursor = Cursor::new(scalar);
	let scalar = Vec::<u64>::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let result = <g1::Config as SWCurveConfig>::mul_affine(&base, &scalar);

	serialize_result(result)
}

/// Compute a scalar multiplication on G2 through arkworks
pub fn mul_affine_g2(base: Vec<u8>, scalar: Vec<u8>) -> Vec<u8> {
	let cursor = Cursor::new(base);
	let base = G2Affine::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let cursor = Cursor::new(scalar);
	let scalar = Vec::<u64>::deserialize_with_mode(cursor, Compress::No, Validate::No).unwrap();

	let result = <g2::Config as SWCurveConfig>::mul_affine(&base, &scalar);

	serialize_result(result)
}

/// Compute a multi scalar multiplication on G! through arkworks
pub fn msm_g1(bases: Vec<Vec<u8>>, scalars: Vec<Vec<u8>>) -> Vec<u8> {
	let bases: Vec<_> = bases
		.iter()
		.map(|a| {
			let cursor = Cursor::new(a);
			<Bls12_381 as Pairing>::G1Affine::deserialize_with_mode(
				cursor,
				Compress::No,
				Validate::No,
			)
			.unwrap()
		})
		.collect();
	let scalars: Vec<_> = scalars
		.iter()
		.map(|a| {
			let cursor = Cursor::new(a);
			<g1::Config as CurveConfig>::ScalarField::deserialize_with_mode(
				cursor,
				Compress::No,
				Validate::No,
			)
			.unwrap()
		})
		.collect();

	let result =
		<<Bls12_381 as Pairing>::G1 as ark_ec::VariableBaseMSM>::msm(&bases, &scalars).unwrap();

	serialize_result(result)
}

/// Compute a multi scalar multiplication on G! through arkworks
pub fn msm_g2(bases: Vec<Vec<u8>>, scalars: Vec<Vec<u8>>) -> Vec<u8> {
	let bases: Vec<_> = bases
		.iter()
		.map(|a| {
			let cursor = Cursor::new(a);
			<Bls12_381 as Pairing>::G2Affine::deserialize_with_mode(
				cursor,
				Compress::No,
				Validate::No,
			)
			.unwrap()
		})
		.collect();
	let scalars: Vec<_> = scalars
		.iter()
		.map(|a| {
			let cursor = Cursor::new(a);
			<g2::Config as CurveConfig>::ScalarField::deserialize_with_mode(
				cursor,
				Compress::No,
				Validate::No,
			)
			.unwrap()
		})
		.collect();

	let result =
		<<Bls12_381 as Pairing>::G2 as ark_ec::VariableBaseMSM>::msm(&bases, &scalars).unwrap();

	serialize_result(result)
}