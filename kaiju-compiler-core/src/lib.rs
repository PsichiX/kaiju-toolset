extern crate kaiju_core as core;

pub mod module_reader;

use crate::core::error::*;
use crate::core::program::*;
use crate::core::validator::*;
use crate::module_reader::*;
use std::collections::HashMap;

pub fn compile_program<V, R>(
    entry_path: &str,
    mut module_reader: R,
    ops_descriptor: &OpsDescriptor,
) -> SimpleResult<Program>
where
    V: DeepValidator,
    R: ModuleReader,
{
    let mut modules = HashMap::new();
    let validator = Validator::new(ops_descriptor);
    load_module::<V, R>(&entry_path, &mut module_reader, &mut modules, &validator)?;
    let mut program =
        Program::from_modules(None, modules.iter().map(|(_, v)| v.clone()).collect())?;
    validator.filter_program::<V>(&mut program);
    if let Some(module) = program.modules.iter().position(|m| m.path == entry_path) {
        if let Some(function) = program.modules[module]
            .functions
            .iter()
            .position(|f| f.header.id == "main")
        {
            program.entry = Some(Entry::new(module, function));
        }
    }
    if let Err(err) = validator.transform_program::<V>(&mut program) {
        return Err(SimpleError::new(format!(
            "Program {}: {}",
            entry_path, err.message
        )));
    }
    if let Err(err) = validator.validate_program::<V>(&program) {
        return Err(SimpleError::new(format!(
            "Program {}: {}",
            entry_path, err.message
        )));
    }
    Ok(program)
}

fn load_module<V, R>(
    relative_path: &str,
    module_reader: &mut R,
    modules: &mut HashMap<String, Module>,
    validator: &Validator,
) -> SimpleResult<String>
where
    V: DeepValidator,
    R: ModuleReader,
{
    let path = module_reader.compose_path(relative_path);
    if modules.contains_key(&path) {
        Ok(path)
    } else if let Some(source) = module_reader.load_module_source(&path) {
        match compile_module(&source) {
            Ok(mut module) => {
                module.path = path.clone();
                module_reader.push_module_path(&path);
                for import in &mut module.imports {
                    import.module =
                        load_module::<V, R>(&import.module, module_reader, modules, validator)?;
                }
                module_reader.pop_module_path();
                modules.insert(path.clone(), module);
                Ok(path)
            }
            Err(err) => Err(SimpleError::new(err.pretty)),
        }
    } else {
        Err(SimpleError::new(format!(
            "Could not load module: {:?}",
            path
        )))
    }
}
