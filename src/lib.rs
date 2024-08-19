use std::{str::FromStr, u64};

use revm_interpreter::{
    opcode::make_instruction_table,
    primitives::{spec_to_generic, Address, Bytecode, Bytes, SpecId, U256},
    Contract, DummyHost, Interpreter, InterpreterAction, SharedMemory,
};
use wasm_bindgen::prelude::*;

impl TryFrom<BigInt> for U256 {
    type Error = js_sys::Error;

    fn try_from(value: BigInt) -> Result<Self, Self::Error> {
        let value_jsstr = value.value.to_string(10)?;
        let value_str = value_jsstr
            .as_string()
            .ok_or_else(|| js_sys::Error::new("Bad BigInt"))?;
        U256::from_str(&value_str)
            .map_err(|_| js_sys::Error::new("BigInt could not be parsed as U256"))
    }
}

pub struct BigInt {
    pub value: js_sys::BigInt,
}

impl From<js_sys::BigInt> for BigInt {
    fn from(value: js_sys::BigInt) -> Self {
        BigInt { value }
    }
}

/// Interpret the given bytecode.
///
/// @param {Uint8Array} bytecode - The bytecode to interpret.
/// @param {Uint8Array} [data] - The data to pass to the contract.
/// @param {bigint} [value] - The value to send to the contract.
/// @param {Uint8Array} [from] - The address of the sender. Default is zero address.
/// @param {Uint8Array} [targetAddress] - The address of the contract. Default is zero address.
/// @param {Uint8Array} [bytecodeAddress] - The address of the bytecode. Default is target address.
/// @param {bigint} [gasLimit] - The gas limit for interpreter. 0 <= gas_limit <= type(uint64).max. Default type(uint64).max.
/// @param {boolean} [staticCall=false] - Whether the call is static. Default is false.
/// @param {string} [specificationName] - The name of the spec to use. Default is LATEST. See: https://github.com/bluealloy/revm/blob/main/crates/primitives/src/specification.rs#L97.
/// @returns {Uint8Array} The result of the interpretation.
#[allow(non_snake_case)]
#[wasm_bindgen(skip_jsdoc)]
pub fn interpret(
    bytecode: &[u8],
    data: Option<Vec<u8>>,
    value: Option<js_sys::BigInt>,
    from: Option<Vec<u8>>,
    targetAddress: Option<Vec<u8>>,
    bytecodeAddress: Option<Vec<u8>>,
    gasLimit: Option<u64>,
    staticCall: Option<bool>,
    specificationName: Option<String>,
) -> Result<Vec<u8>, js_sys::Error> {
    let contract = Contract::new(
        data.map_or_else(|| Bytes::default(), |v| Bytes::from_iter(v)),
        Bytecode::new_raw(Bytes::from_iter(bytecode)),
        None,
        targetAddress.map_or_else(
            || Ok(Address::ZERO),
            |v| {
                v.as_slice()
                    .try_into()
                    .map_err(|_| js_sys::Error::new("Bad target address"))
            },
        )?,
        match bytecodeAddress {
            Some(v) => Some(
                v.as_slice()
                    .try_into()
                    .map_err(|_| js_sys::Error::new("Bad bytecode address"))?,
            ),
            None => None,
        },
        from.map_or_else(
            || Ok(Address::ZERO),
            |v| {
                v.as_slice()
                    .try_into()
                    .map_err(|_| js_sys::Error::new("Bad from address"))
            },
        )?,
        value.map_or_else(|| Ok(U256::ZERO), |v| BigInt::from(v).try_into())?,
    );

    let mut interpreter = Interpreter::new(
        contract,
        gasLimit.unwrap_or_else(|| u64::MAX),
        staticCall.unwrap_or(false),
    );

    let mut host = DummyHost::default();
    let spec_id = specificationName.map_or_else(|| SpecId::LATEST, |v| SpecId::from(v.as_str()));
    let table = spec_to_generic!(spec_id, &make_instruction_table::<DummyHost, SPEC>());

    if let InterpreterAction::Return { result } =
        interpreter.run(SharedMemory::new(), table, &mut host)
    {
        Ok(result.output.to_vec())
    } else {
        Err(js_sys::Error::new("Bad interpreter action"))
    }
}
