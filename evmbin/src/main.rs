// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

//! Parity EVM interpreter binary.

#![warn(missing_docs)]

extern crate common_types as types;
extern crate ethcore;
extern crate ethjson;
extern crate rustc_hex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
/*
#[macro_use]
extern crate serde_json;
*/
extern crate docopt;
extern crate parity_bytes as bytes;
extern crate ethereum_types;
extern crate vm;
extern crate evm;
extern crate panic_hook;
extern crate env_logger;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[cfg(test)]
extern crate tempdir;

use std::time::{Instant};
use std::sync::Arc;
use std::{fmt, fs};
use docopt::Docopt;
use rustc_hex::FromHex;
use ethereum_types::{U256, Address};
use bytes::Bytes;
use bytes::ToPretty;
use vm::ActionParams;

//mod info;
//mod display;

const USAGE: &'static str = r#"
EVM implementation for Parity.
  Copyright 2015-2019 Parity Technologies (UK) Ltd.

Usage:
    parity-evm [options]
    parity-evm [-h | --help]

Transaction options:
    --code-file CODEFILE    Read contract code from file as hex (without 0x).
    --code CODE        Contract code as hex (without 0x).
    --to ADDRESS       Recipient address (without 0x).
    --from ADDRESS     Sender address (without 0x).
    --input DATA       Input data as hex (without 0x).
    --expected DATA    Expected return data as hex (without 0x).
    --gas GAS          Supplied gas as hex (without 0x).
    --gas-price WEI    Supplied gas price as hex (without 0x).

    -h, --help         Display this message and exit.
"#;

fn main() {
	panic_hook::set_abort();
	env_logger::init();

	let args: Args = Docopt::new(USAGE).and_then(|d| d.deserialize()).unwrap_or_else(|e| e.exit());

	run_call(args)
}

fn run_call(args: Args) {
	let _from = arg(args.from(), "--from");
	let _to = arg(args.to(), "--to");
	let code_file = arg(args.code_file(), "--code-file");
	let code = arg(args.code(), "--code");
	let _gas = arg(args.gas(), "--gas");
	let _gas_price = arg(args.gas_price(), "--gas-price");
	let calldata = arg(args.data(), "--input");
    let expected = arg(args.expected(), "--expected");

	if code.is_none() && code_file.is_none() {
		die("Either --code or --code-file is required.");
	}

    if expected.is_none() {
        die("Expected return data --expected is required.");
    }

    let code = code_file.unwrap();
    let expected_return = expected.unwrap().clone();

    //let gas = U256::from(::std::usize::MAX);
    let gas = U256::from(1000000);

    let mut params = ActionParams::default();
    params.gas = gas;
    params.code = Some(Arc::new(code.clone()));
    params.data = calldata.clone();

    let spec = ethcore::ethereum::new_constantinople_test();
    let mut test_client = ethcore::client::EvmTestClient::new(&spec).unwrap();
    let call_result = test_client.call(params, &mut ethcore::trace::NoopTracer, &mut ethcore::trace::NoopVMTracer).unwrap();
    let return_data = call_result.return_data.to_vec().to_hex();
    println!("return_data: {:?}", return_data);
    println!("gas used: {:?}", gas - call_result.gas_left);

    if return_data != expected_return {
        println!("Wrong return data!  got: {:?}   expected: {:?}", return_data, expected_return);
        die("wrong return data.");
    }


    let iterations = 100;
    let mut total_duration = std::time::Duration::new(0, 0);

    for _i in 0..iterations {
        let mut params = ActionParams::default();
        params.gas = gas;
        params.code = Some(Arc::new(code.clone()));
        params.data = calldata.clone();

        let spec = ethcore::ethereum::new_constantinople_test();
        let mut test_client = ethcore::client::EvmTestClient::new(&spec).unwrap();

        let start_run = Instant::now();

        let _result = test_client.call(params, &mut ethcore::trace::NoopTracer, &mut ethcore::trace::NoopVMTracer).unwrap();

        let run_duration = start_run.elapsed();
        total_duration = total_duration + run_duration;
    }

    let avg_duration = total_duration / iterations;
    println!("code avg run time: {:?}", avg_duration);

}




#[derive(Debug, Deserialize)]
struct Args {
    flag_code_file: Option<String>,
	flag_only: Option<String>,
	flag_from: Option<String>,
	flag_to: Option<String>,
	flag_code: Option<String>,
	flag_gas: Option<String>,
	flag_gas_price: Option<String>,
	flag_input: Option<String>,
    flag_expected: Option<String>,
}

impl Args {
	pub fn gas(&self) -> Result<U256, String> {
		match self.flag_gas {
			Some(ref gas) => gas.parse().map_err(to_string),
			None => Ok(U256::from(u64::max_value())),
		}
	}

	pub fn gas_price(&self) -> Result<U256, String> {
		match self.flag_gas_price {
			Some(ref gas_price) => gas_price.parse().map_err(to_string),
			None => Ok(U256::zero()),
		}
	}

	pub fn from(&self) -> Result<Address, String> {
		match self.flag_from {
			Some(ref from) => from.parse().map_err(to_string),
			None => Ok(Address::default()),
		}
	}

	pub fn to(&self) -> Result<Address, String> {
		match self.flag_to {
			Some(ref to) => to.parse().map_err(to_string),
			None => Ok(Address::default()),
		}
	}

	pub fn code(&self) -> Result<Option<Bytes>, String> {
		match self.flag_code {
			Some(ref code) => code.from_hex().map(Some).map_err(to_string),
			None => Ok(None),
		}
	}

	pub fn data(&self) -> Result<Option<Bytes>, String> {
		match self.flag_input {
			Some(ref input) => input.from_hex().map_err(to_string).map(Some),
			None => Ok(None),
		}
	}

	pub fn expected(&self) -> Result<Option<String>, String> {
		match self.flag_expected {
			Some(ref expected) => expected.parse().map_err(to_string).map(Some),
			None => Ok(None),
		}
	}

    pub fn code_file(&self) -> Result<Option<Bytes>, String> {
        match self.flag_code_file {
            Some(ref filename) => {
                let code_hex = fs::read_to_string(filename).unwrap();
                println!("code_hex length: {:?}", code_hex.len());
                code_hex.from_hex().map_err(to_string).map(Some)
            },
            None => Ok(None),
        }
    }

}

fn arg<T>(v: Result<T, String>, param: &str) -> T {
	v.unwrap_or_else(|e| die(format!("Invalid {}: {}", param, e)))
}

fn to_string<T: fmt::Display>(msg: T) -> String {
	format!("{}", msg)
}

fn die<T: fmt::Display>(msg: T) -> ! {
	println!("{}", msg);
	::std::process::exit(-1)
}

#[cfg(test)]
mod tests {
	use docopt::Docopt;
	use super::{Args, USAGE};

	fn run<T: AsRef<str>>(args: &[T]) -> Args {
		Docopt::new(USAGE).and_then(|d| d.argv(args.into_iter()).deserialize()).unwrap()
	}

	#[test]
	fn should_parse_all_the_options() {
		let args = run(&[
			"parity-evm",
			"--gas", "1",
			"--gas-price", "2",
			"--from", "0000000000000000000000000000000000000003",
			"--to", "0000000000000000000000000000000000000004",
			"--code", "05",
			"--input", "06",
		]);

		assert_eq!(args.gas(), Ok(1.into()));
		assert_eq!(args.gas_price(), Ok(2.into()));
		assert_eq!(args.from(), Ok(3.into()));
		assert_eq!(args.to(), Ok(4.into()));
		assert_eq!(args.code(), Ok(Some(vec![05])));
		assert_eq!(args.data(), Ok(Some(vec![06])));
	}

}
