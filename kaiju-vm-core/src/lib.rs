extern crate itertools;
extern crate kaiju_compiler_core as compiler_core;
extern crate kaiju_core as core;

pub mod processor;
pub mod state;
pub mod vm;

use crate::vm::Vm;
use core::error::*;
use std::ffi::CString;

pub fn load_cstring(address: usize, vm: &Vm) -> SimpleResult<String> {
    let p = vm.state().load_data::<usize>(address)?;
    let bytes = vm.state().load_bytes_while_non_zero(p);
    match CString::new(bytes).unwrap().into_string() {
        Ok(v) => Ok(v),
        Err(err) => Err(SimpleError::new(format!("{}", err))),
    }
}

pub fn store_cstring(value: &str, address: usize, vm: &mut Vm) -> SimpleResult<()> {
    if let Ok(ref cs) = CString::new(value) {
        let bytes = cs.as_bytes_with_nul();
        let v = vm.state_mut().stack_push_bytes(bytes)?;
        vm.state_mut().store_data(address, &v.address)?;
        Ok(())
    } else {
        Err(SimpleError::new(format!(
            "Could not store string that is not C-compatible: '{}'",
            value
        )))
    }
}
