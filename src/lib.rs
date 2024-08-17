use std::str::FromStr;

use revm_interpreter::{
    opcode::{make_instruction_table, InstructionTable},
    primitives::{Bytecode, Bytes, ShanghaiSpec, U256},
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

#[wasm_bindgen]
pub fn interpret(
    from: &[u8],
    data: &[u8],
    value: js_sys::BigInt,
    target_address: &[u8],
    bytecode: &[u8],
    gas_limit: u64,
    static_call: Option<bool>,
) -> Result<Vec<u8>, js_sys::Error> {
    let contract = Contract::new(
        Bytes::from_iter(data),
        Bytecode::new_raw(Bytes::from_iter(bytecode)),
        None,
        target_address
            .try_into()
            .map_err(|_| js_sys::Error::new("Bad target address"))?,
        None,
        from.try_into()
            .map_err(|_| js_sys::Error::new("Bad from address"))?,
        BigInt::from(value).try_into()?,
    );
    let mut interpreter = Interpreter::new(contract, gas_limit, static_call.unwrap_or(false));

    let mut host = DummyHost::default();
    let table: &InstructionTable<DummyHost> = &make_instruction_table::<DummyHost, ShanghaiSpec>();

    if let InterpreterAction::Return { result } =
        interpreter.run(SharedMemory::new(), table, &mut host)
    {
        Ok(result.output.to_vec())
    } else {
        Err(js_sys::Error::new("Bad interpreter action"))
    }
}
