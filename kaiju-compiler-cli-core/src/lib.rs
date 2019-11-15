extern crate kaiju_compiler_core as compiler_core;
extern crate kaiju_core as core;
extern crate libloading;
#[macro_use]
extern crate lazy_static;
extern crate relative_path;

pub mod external_deep_validator;
pub mod fs_module_reader;

use crate::core::assembly::*;
use crate::core::error::*;
use crate::core::program::*;
use crate::core::validator::*;
use crate::fs_module_reader::*;
use std::fs::{read_to_string, write};

pub fn load_opdescs(paths: &[String]) -> SimpleResult<OpsDescriptor> {
    if paths.is_empty() {
        Ok(OpsDescriptor::default())
    } else {
        let mut descs = vec![];
        for path in paths {
            match read_to_string(&path) {
                Ok(desc) => match compile_ops_descriptor(&desc) {
                    Ok(desc) => descs.push(desc),
                    Err(err) => {
                        return Err(SimpleError::new(format!("{:?}: {}", path, err.pretty)))
                    }
                },
                Err(err) => return Err(SimpleError::new(format!("{:?}: {}", path, err))),
            }
        }
        Ok(OpsDescriptor::merge(&descs))
    }
}

pub fn compile_program<V>(input: &str, opsdesc: &OpsDescriptor) -> SimpleResult<Program>
where
    V: DeepValidator,
{
    compiler_core::compile_program::<V, _>(input, FsModuleReader::default(), opsdesc)
}

pub fn compile_program_and_write_pst<V>(
    input: &str,
    output: &str,
    opsdesc: &OpsDescriptor,
    pretty: bool,
) -> SimpleResult<()>
where
    V: DeepValidator,
{
    let program = compile_program::<V>(input, opsdesc)?;
    match program.to_json(pretty) {
        Ok(json) => {
            if let Err(err) = write(output, &json) {
                Err(SimpleError::new(format!("{:?}: {}", output, err)))
            } else {
                Ok(())
            }
        }
        Err(err) => Err(SimpleError::new(format!("{:?}: {}", output, err))),
    }
}

pub fn compile_program_and_write_bin<V>(
    input: &str,
    output: &str,
    opsdesc: &OpsDescriptor,
) -> SimpleResult<()>
where
    V: DeepValidator,
{
    let program = compile_program::<V>(input, opsdesc)?;
    match encode_assembly(&program, opsdesc) {
        Ok(bytes) => {
            if let Err(err) = write(output, &bytes) {
                Err(SimpleError::new(format!("{:?}: {}", output, err)))
            } else {
                Ok(())
            }
        }
        Err(err) => Err(SimpleError::new(format!("{:?}: {}", output, err.message))),
    }
}
