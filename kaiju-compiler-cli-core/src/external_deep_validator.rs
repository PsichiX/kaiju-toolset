use crate::core::error::*;
use crate::core::program::*;
use crate::core::validator::*;
use libloading::{Library, Symbol};
use std::sync::Mutex;

type FuncOnFilterModule = fn(&Module, &Program, &Validator) -> bool;
type FuncOnFilterStruct = fn(&Struct, &Module, &Program, &Validator) -> bool;
type FuncOnFilterFunction = fn(&Function, &Module, &Program, &Validator) -> bool;
type FuncOnFilterOp = fn(&Operation, &Function, &Module, &Program, &Validator) -> bool;
type FuncOnValidateProgram = fn(&Program, &Validator) -> SimpleResult<()>;
type FuncOnValidateModule = fn(&Module, &Program, &Validator) -> SimpleResult<()>;
type FuncOnValidateOp =
    fn(&Operation, &Function, &Module, &Program, &Rule, &Validator) -> SimpleResult<()>;

lazy_static! {
    static ref LIBS: Mutex<Vec<Library>> = Mutex::new(vec![]);
}

pub fn load_validator(path: &str) -> SimpleResult<()> {
    let lib = match Library::new(path) {
        Ok(lib) => Ok(lib),
        Err(err) => Err(SimpleError::new(format!("{}: {}", path, err))),
    }?;
    LIBS.lock().unwrap().push(lib);
    Ok(())
}

pub struct ExternalDeepValidator {}

impl DeepValidator for ExternalDeepValidator {
    fn filter_module(module: &Module, program: &Program, validator: &Validator) -> bool {
        LIBS.lock().unwrap().iter().all(|lib| unsafe {
            match lib.get(b"on_filter_module") {
                Ok(f) => {
                    let cb: Symbol<FuncOnFilterModule> = f;
                    cb(module, program, validator)
                }
                Err(_) => true,
            }
        })
    }

    fn filter_struct(
        struct_: &Struct,
        module: &Module,
        program: &Program,
        validator: &Validator,
    ) -> bool {
        LIBS.lock().unwrap().iter().all(|lib| unsafe {
            match lib.get(b"on_filter_struct") {
                Ok(f) => {
                    let cb: Symbol<FuncOnFilterStruct> = f;
                    cb(struct_, module, program, validator)
                }
                Err(_) => true,
            }
        })
    }

    fn filter_function(
        function: &Function,
        module: &Module,
        program: &Program,
        validator: &Validator,
    ) -> bool {
        LIBS.lock().unwrap().iter().all(|lib| unsafe {
            match lib.get(b"on_filter_function") {
                Ok(f) => {
                    let cb: Symbol<FuncOnFilterFunction> = f;
                    cb(function, module, program, validator)
                }
                Err(_) => true,
            }
        })
    }

    fn filter_op(
        op: &Operation,
        function: &Function,
        module: &Module,
        program: &Program,
        validator: &Validator,
    ) -> bool {
        LIBS.lock().unwrap().iter().all(|lib| unsafe {
            match lib.get(b"on_filter_op") {
                Ok(f) => {
                    let cb: Symbol<FuncOnFilterOp> = f;
                    cb(op, function, module, program, validator)
                }
                Err(_) => true,
            }
        })
    }

    fn validate_program(program: &Program, validator: &Validator) -> SimpleResult<()> {
        let libs = LIBS.lock().unwrap();
        for lib in libs.iter() {
            unsafe {
                match lib.get(b"on_validate_program") {
                    Ok(f) => {
                        let cb: Symbol<FuncOnValidateProgram> = f;
                        cb(program, validator)
                    }
                    Err(_) => Ok(()),
                }
            }?;
        }
        Ok(())
    }

    fn validate_module(
        module: &Module,
        program: &Program,
        validator: &Validator,
    ) -> SimpleResult<()> {
        let libs = LIBS.lock().unwrap();
        for lib in libs.iter() {
            unsafe {
                match lib.get(b"on_validate_module") {
                    Ok(f) => {
                        let cb: Symbol<FuncOnValidateModule> = f;
                        cb(module, program, validator)
                    }
                    Err(_) => Ok(()),
                }
            }?;
        }
        Ok(())
    }

    fn validate_op(
        op: &Operation,
        function: &Function,
        module: &Module,
        program: &Program,
        rule: &Rule,
        validator: &Validator,
    ) -> SimpleResult<()> {
        let libs = LIBS.lock().unwrap();
        for lib in libs.iter() {
            unsafe {
                match lib.get(b"on_validate_op") {
                    Ok(f) => {
                        let cb: Symbol<FuncOnValidateOp> = f;
                        cb(op, function, module, program, rule, validator)
                    }
                    Err(_) => Ok(()),
                }
            }?;
        }
        Ok(())
    }

    fn transform_module(
        module: Module,
        _program: &Program,
        _validator: &Validator,
    ) -> SimpleResult<Module> {
        transform_module_auto_types(module)
    }
}
