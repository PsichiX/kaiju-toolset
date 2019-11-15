use crate::vm::Vm;
use core::error::*;

#[derive(Debug, Copy, Clone)]
pub enum OpAction {
    None,
    GoTo(usize),
    Return,
}

pub trait Processor {
    fn process_op(
        _op: &String,
        _params: &[usize],
        _targets: &[usize],
        _vm: &mut Vm,
    ) -> SimpleResult<OpAction> {
        unimplemented!()
    }
}

pub struct EmptyProcessor {}
impl Processor for EmptyProcessor {}
