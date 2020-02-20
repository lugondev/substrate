// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! ImOnline pallet benchmarking.

use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{
	BenchmarkResults, BenchmarkParameter, selected_benchmark, benchmarking, Benchmarking,
    BenchmarkingSetup,
};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{Bounded, Dispatchable};
use sp_application_crypto::{key_types,RuntimePublic, RuntimeAppPublic};

use codec::{Encode, Decode};

use crate::Module as ImOnline;


fn block_number<T: Trait>() -> T::BlockNumber {
	// let entropy = (b"whatsoever", 0u32).using_encoded(blake2_256);
	// <T::BlockNumber as Decode>::decode(&mut &entropy).expect("decode of heartbeat should work just fine")
	// // unimplemented!("a random block number construction is a TODO")
	27u32.into()
}


fn heartbeat<T: Trait>() -> crate::Heartbeat<T::BlockNumber> {
	crate::Heartbeat::<T::BlockNumber> {
		block_number : <T::BlockNumber>::from(88u32),
		session_index : 56u32 as SessionIndex,
		authority_index : 14u32 as AuthIndex,
		network_state : sp_core::offchain::OpaqueNetworkState {
			peer_id : sp_core::offchain::OpaquePeerId(vec![0u8, 2u8]),
			external_addresses : vec![
				sp_core::offchain::OpaqueMultiaddr (
					vec![192u8,168u8,1u8,121u8]
				)
			]
		}
	}
}

// Benchmark `add_registrar` extrinsic.
struct Heartbeat;
impl<T: Trait> BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>> for Heartbeat {
	fn components(&self) -> Vec<(BenchmarkParameter, u32, u32)> {
		vec![
			// unused
			(BenchmarkParameter::X, 1, 10),
		]
	}

	fn instance(&self, components: &[(BenchmarkParameter, u32)])
		-> Result<(crate::Call<T>, RawOrigin<T::AccountId>), &'static str>
	{
		// let signature = Default::default();
		// let heartbeat_data = pallet_im_online::Heartbeat {
		// 	block_number: 1,
		// 	network_state: Default::default(),
		// 	session_index: 1,
		// 	authority_index: 0,
		// };

		// let call = pallet_im_online::Call::heartbeat(heartbeat_data, signature);
		// <SubmitTransaction as SubmitUnsignedTransaction<Runtime, Call>>
		// 	::submit_unsigned(call)
		// 	.unwrap();

		// assert_eq!(state.read().transactions.len(), 1)
		// });


		let seed = Some(vec![1,2,3,4]);

		let hb = heartbeat::<T>();
		let raw = hb.encode();
		let pair = <<T as Trait>::AuthorityId as RuntimeAppPublic>::generate_pair(seed);
		let signature = pair.sign(&raw).expect("Failed to calculate signature");

		Ok((crate::Call::<T>::heartbeat(hb, signature), RawOrigin::None))
	}
}


// The list of available benchmarks for this pallet.
enum SelectedBenchmark {
	Heartbeat,
}

// Allow us to select a benchmark from the list of available benchmarks.
impl<T: Trait> BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>> for SelectedBenchmark {
	fn components(&self) -> Vec<(BenchmarkParameter, u32, u32)> {
		match self {
			Self::Heartbeat => <Heartbeat as BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>>>::components(&Heartbeat),
		}
	}

	fn instance(&self, components: &[(BenchmarkParameter, u32)])
		-> Result<(crate::Call<T>, RawOrigin<T::AccountId>), &'static str>
	{
		match self {
			Self::Heartbeat => <Heartbeat as BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>>>::instance(&Heartbeat, components),
		}
	}
}

impl<T: Trait> Benchmarking<BenchmarkResults> for Module<T> {
	fn run_benchmark(extrinsic: Vec<u8>, steps: u32, repeat: u32) -> Result<Vec<BenchmarkResults>, &'static str> {
		// Map the input to the selected benchmark.
		let selected_benchmark = match extrinsic.as_slice() {
			b"heartbeat" => SelectedBenchmark::Heartbeat,
			_ => return Err("Could not find extrinsic."),
		};

		// Warm up the DB
		benchmarking::commit_db();
		benchmarking::wipe_db();

		// first one is set_identity.
		let components = <SelectedBenchmark as BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>>>::components(&selected_benchmark);
		// results go here
		let mut results: Vec<BenchmarkResults> = Vec::new();
		// Select the component we will be benchmarking. Each component will be benchmarked.
		for (name, low, high) in components.iter() {
			// Create up to `STEPS` steps for that component between high and low.
			let step_size = ((high - low) / steps).max(1);
			let num_of_steps = (high - low) / step_size;
			for s in 0..num_of_steps {
				// This is the value we will be testing for component `name`
				let component_value = low + step_size * s;

				// Select the mid value for all the other components.
				let c: Vec<(BenchmarkParameter, u32)> = components.iter()
					.map(|(n, l, h)|
						(*n, if n == name { component_value } else { (h - l) / 2 + l })
					).collect();

				// Run the benchmark `repeat` times.
				for _ in 0..repeat {
					// Set up the externalities environment for the setup we want to benchmark.
					let (call, caller) = <SelectedBenchmark as BenchmarkingSetup<T, crate::Call<T>, RawOrigin<T::AccountId>>>::instance(&selected_benchmark, &c)?;
					// Commit the externalities to the database, flushing the DB cache.
					// This will enable worst case scenario for reading from the database.
					benchmarking::commit_db();
					// Run the benchmark.
					let start = benchmarking::current_time();
					call.dispatch(caller.into())?;
					let finish = benchmarking::current_time();
					let elapsed = finish - start;
					results.push((c.clone(), elapsed));
					// Wipe the DB back to the genesis state.
					benchmarking::wipe_db();
				}
			}
		}
		return Ok(results);
	}
}
