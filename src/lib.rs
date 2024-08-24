#![no_std]

extern crate alloc;

use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};
use core::str::FromStr;
use serde::Deserialize;
use tsify_next::Tsify;

use revm_interpreter::{
    opcode::make_instruction_table,
    primitives::{spec_to_generic, Bytecode, SpecId, U256},
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

impl TryFrom<JsValue> for BigInt {
    type Error = js_sys::Error;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if value.is_bigint() {
            Ok(BigInt {
                value: value.unchecked_into(),
            })
        } else if value.is_undefined() {
            Ok(BigInt {
                value: js_sys::BigInt::default(),
            })
        } else {
            Err(js_sys::Error::new("Value is not a BigInt or undefined"))
        }
    }
}

#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi, large_number_types_as_bigints)]
#[serde(rename_all = "camelCase")]
pub struct InterpretParams {
    #[tsify(type = "Uint8Array")]
    #[serde(with = "serde_bytes")]
    /// The bytecode to interpret.
    bytecode: Vec<u8>,
    #[tsify(type = "Uint8Array")]
    #[serde(default, with = "serde_bytes")]
    /// The data to pass to the contract.
    data: Vec<u8>,
    #[tsify(type = "bigint")]
    #[serde(default, with = "serde_wasm_bindgen::preserve")]
    /// The value to send to the contract.
    value: JsValue,
    #[tsify(type = "Uint8Array")]
    #[serde(default, with = "serde_bytes")]
    /// The address of the sender. Default is zero address.
    from: Option<[u8; 20]>,
    #[tsify(type = "Uint8Array")]
    #[serde(default, with = "serde_bytes")]
    /// The address of the contract. Default is zero address.
    target_address: Option<[u8; 20]>,
    #[tsify(type = "Uint8Array")]
    #[serde(default, with = "serde_bytes")]
    /// The address of the bytecode. Default is target address.
    bytecode_address: Option<[u8; 20]>,
    #[tsify(type = "bigint")]
    #[serde(default)]
    /// The gas limit for interpreter. 0 <= gas_limit <= type(uint64).max. Default type(uint64).max.
    gas_limit: Option<u64>,
    #[tsify(type = "boolean")]
    #[serde(default)]
    /// Whether the call is static. Default is false.
    static_call: Option<bool>,
    #[tsify(
        type = "'Frontier' | 'Homestead' | 'Tangerine' | 'Spurious' | 'Byzantium' | 'Constantinople' | 'Petersburg' | 'Istanbul' | 'MuirGlacier' | 'Berlin' | 'London' | 'Merge' | 'Shanghai' | 'Cancun' | 'Prague' | 'PragueEOF'"
    )]
    #[serde(default)]
    /// The name of the spec to use. Default is LATEST. See: https://github.com/bluealloy/revm/blob/main/crates/primitives/src/specification.rs#L97.
    specification_name: Option<String>,
}

/// Interpret the given bytecode.
///
/// @param {InterpretParams} params - The parameters interpreter parameters.
/// @returns {Uint8Array} The result of the interpretation.
#[allow(non_snake_case)]
#[wasm_bindgen(skip_jsdoc)]
pub fn interpret(params: InterpretParams) -> Result<Vec<u8>, js_sys::Error> {
    let contract = Contract::new(
        params.data.into(),
        Bytecode::new_raw(params.bytecode.into()),
        None,
        params.target_address.unwrap_or_default().into(),
        params.bytecode_address.map(|v| v.into()),
        params.from.unwrap_or_default().into(),
        BigInt::try_from(params.value)?.try_into()?,
    );

    let mut interpreter = Interpreter::new(
        contract,
        params.gas_limit.unwrap_or_else(|| u64::MAX),
        params.static_call.unwrap_or_default(),
    );

    let mut host = DummyHost::default();
    let spec_id = params
        .specification_name
        .map_or_else(|| SpecId::LATEST, |v| SpecId::from(v.as_str()));
    let table = spec_to_generic!(spec_id, &make_instruction_table::<DummyHost, SPEC>());

    if let InterpreterAction::Return { result } =
        interpreter.run(SharedMemory::new(), table, &mut host)
    {
        Ok(result.output.to_vec())
    } else {
        Err(js_sys::Error::new("Bad interpreter action"))
    }
}
