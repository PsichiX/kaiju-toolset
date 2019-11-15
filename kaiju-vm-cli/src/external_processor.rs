use core::error::*;
use libloading::{Library, Symbol};
use std::sync::Mutex;
use vm_core::processor::{OpAction, Processor};
use vm_core::vm::Vm;

type FuncOnProcessOp = fn(&String, &[usize], &[usize], &mut Vm) -> SimpleResult<OpAction>;

lazy_static! {
    static ref LIB: Mutex<Option<Library>> = Mutex::new(None);
}

pub fn load_processor(path: &str) -> SimpleResult<()> {
    let mut lib = LIB.lock().unwrap();
    *lib = Some(match Library::new(path) {
        Ok(lib) => Ok(lib),
        Err(err) => Err(SimpleError::new(format!("{}: {}", path, err))),
    }?);
    Ok(())
}

pub struct ExternalProcessor {}

impl Processor for ExternalProcessor {
    fn process_op(
        op: &String,
        params: &[usize],
        targets: &[usize],
        vm: &mut Vm,
    ) -> SimpleResult<OpAction> {
        if let Some(ref lib) = *LIB.lock().unwrap() {
            unsafe {
                match lib.get(b"on_process_op") {
                    Ok(f) => {
                        let cb: Symbol<FuncOnProcessOp> = f;
                        cb(op, params, targets, vm)
                    }
                    Err(_) => Err(SimpleError::new(
                        "There is no external processor `on_process_op` function to call"
                            .to_owned(),
                    )),
                }
            }
        } else {
            Err(SimpleError::new(
                "There is no external processor loaded".to_owned(),
            ))
        }
    }
}
