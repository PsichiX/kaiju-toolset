extern crate clap;
extern crate kaiju_core as core;
extern crate kaiju_vm_core as vm_core;
#[macro_use]
extern crate lazy_static;
extern crate kaiju_compiler_cli_core as compiler_cli_core;
extern crate libloading;

pub mod external_processor;

use crate::external_processor::load_processor;
use crate::external_processor::ExternalProcessor;
use clap::{App, Arg, ArgGroup};
use compiler_cli_core::external_deep_validator::load_validator;
use compiler_cli_core::external_deep_validator::ExternalDeepValidator;
use compiler_cli_core::fs_module_reader::FsModuleReader;
use compiler_cli_core::load_opdescs;
use core::program::OpsDescriptor;
use std::fs::read;
use std::path::Path;
use vm_core::vm::Vm;

fn main() {
    let matches = App::new("Kaiju Virtual Machine CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Kaiju input module file (*.kj)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("binary")
                .short("b")
                .long("binary")
                .value_name("FILE")
                .help("Kaiju input module binary (*.kjb)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("opdesc")
                .short("d")
                .long("opdesc")
                .value_name("FILE")
                .help("Kaiju ops descriptor file (*.kjo)")
                .required(false)
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("validator")
                .short("v")
                .long("validator")
                .value_name("FILE")
                .help("Kaiju deep validator plugin (*.dll|*.so)")
                .required(false)
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("processor")
                .short("p")
                .long("processor")
                .value_name("FILE")
                .help("Kaiju processor plugin (*.dll|*.so)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("entry")
                .short("e")
                .long("func")
                .value_name("ID")
                .help("Entry function ID")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("memsize")
                .short("m")
                .long("memsize")
                .value_name("BYTESIZE")
                .help("Memory size in bytes")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("stacksize")
                .short("s")
                .long("stacksize")
                .value_name("BYTESIZE")
                .help("Stack size in bytes")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dialect")
                .short("l")
                .long("dialect")
                .value_name("NAME")
                .help("Kaiju language dialect name")
                .takes_value(true),
        )
        .group(
            ArgGroup::with_name("inputs")
                .args(&["input", "binary"])
                .required(true),
        )
        .group(
            ArgGroup::with_name("mode")
                .args(&["processor", "dialect"])
                .required(true),
        )
        .get_matches();

    let (dialect_opdesc, dialect_validator, dialect_processor) = if let Some(dialect) =
        matches.value_of("dialect")
    {
        let dialects_path = ::std::env::var("KAIJU_DIALECTS").unwrap_or_else(|_| ".".to_owned());
        let path_opdesc = format!("{}/{}/descriptor.kjo", dialects_path, dialect);
        let path_validator = format!("{}/{}/validator.dll", dialects_path, dialect);
        let path_processor = format!("{}/{}/processor.dll", dialects_path, dialect);
        let opdesc = if Path::new(&path_opdesc).exists() {
            Some(path_opdesc)
        } else {
            None
        };
        let validator = if Path::new(&path_validator).exists() {
            Some(path_validator)
        } else {
            None
        };
        let processor = if Path::new(&path_processor).exists() {
            Some(path_processor)
        } else {
            None
        };
        (opdesc, validator, processor)
    } else {
        (None, None, None)
    };
    let opsdesc = if let Some(opdescs) = matches.values_of("opdesc") {
        let paths = opdescs.map(String::from).collect::<Vec<String>>();
        load_opdescs(&paths).unwrap()
    } else if let Some(opdesc) = dialect_opdesc {
        load_opdescs(&[opdesc]).unwrap()
    } else {
        OpsDescriptor::default()
    };
    if let Some(validators) = matches.values_of("validator") {
        for validator in validators {
            if let Err(err) = load_validator(&validator) {
                eprintln!("{}", err.message);
                ::std::process::exit(1);
            }
        }
    } else if let Some(validator) = dialect_validator {
        if let Err(err) = load_validator(&validator) {
            eprintln!("{}", err.message);
            ::std::process::exit(1);
        }
    }
    let processor = if let Some(processor) = dialect_processor {
        processor
    } else {
        matches.value_of("processor").unwrap().to_owned()
    };
    if let Err(err) = load_processor(&processor) {
        eprintln!("{}", err.message);
        ::std::process::exit(1);
    }
    let entry = if let Some(entry) = matches.value_of("entry") {
        entry
    } else {
        "main"
    };
    let memsize = if let Some(memsize) = matches.value_of("memsize") {
        memsize.parse().unwrap()
    } else {
        1024
    };
    let stacksize = if let Some(stacksize) = matches.value_of("stacksize") {
        stacksize.parse().unwrap()
    } else {
        256
    };
    let mut vm = if let Some(input) = matches.value_of("input") {
        match Vm::from_source::<ExternalDeepValidator, FsModuleReader>(
            &input,
            FsModuleReader::default(),
            &opsdesc,
            stacksize,
            memsize,
        ) {
            Ok(vm) => vm,
            Err(err) => {
                eprintln!("{}", err.message);
                ::std::process::exit(1);
            }
        }
    } else if let Some(binary) = matches.value_of("binary") {
        match read(&binary) {
            Ok(bytes) => match Vm::from_bytes(bytes, stacksize, memsize) {
                Ok(assembly) => assembly,
                Err(err) => {
                    eprintln!("{}", err.message);
                    ::std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("{}", err);
                ::std::process::exit(1);
            }
        }
    } else {
        unreachable!();
    };
    if let Err(err) = vm.run::<ExternalProcessor>(entry) {
        eprintln!("{}", err.message);
        ::std::process::exit(1);
    }
}
