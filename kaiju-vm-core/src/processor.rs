use crate::vm::Vm;
use core::error::*;

#[derive(Debug, Copy, Clone)]
pub enum OpAction {
    None,
    GoTo(usize),
    Return,
}

pub trait Processor {
    #[allow(clippy::ptr_arg)]
    fn process_op(
        _op: &String,
        _params: &Vec<usize>,
        _targets: &Vec<usize>,
        _vm: &mut Vm,
    ) -> SimpleResult<OpAction> {
        unimplemented!()
    }
}

pub struct EmptyProcessor {}
impl Processor for EmptyProcessor {}
