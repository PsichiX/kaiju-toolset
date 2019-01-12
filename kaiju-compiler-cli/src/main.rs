extern crate clap;
extern crate kaiju_compiler_cli_core as compiler_cli_core;
extern crate kaiju_compiler_core as compiler_core;
extern crate kaiju_core as core;

use crate::compiler_cli_core::external_deep_validator::*;
use crate::compiler_cli_core::*;
use crate::core::program::OpsDescriptor;
use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("Kaiju Compiler CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            SubCommand::with_name("bin")
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about("Build binary from source")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .value_name("FILE")
                        .help("Kaiju input module file (*.kj)")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("FILE")
                        .help("Kaiju output binary file (*.kjb)")
                        .required(true)
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
                ),
        )
        .subcommand(
            SubCommand::with_name("pst")
                .version(env!("CARGO_PKG_VERSION"))
                .author(env!("CARGO_PKG_AUTHORS"))
                .about("Build program structure tree from source")
                .arg(
                    Arg::with_name("input")
                        .short("i")
                        .long("input")
                        .value_name("FILE")
                        .help("Kaiju input module file (*.kj)")
                        .required(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("output")
                        .short("o")
                        .long("output")
                        .value_name("FILE")
                        .help("Kaiju output binary file (*.json)")
                        .required(true)
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
                    Arg::with_name("pretty")
                        .short("p")
                        .long("pretty")
                        .help("Make output pretty")
                        .required(false),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("bin", Some(matches)) => {
            let input = matches.value_of("input").unwrap();
            let output = matches.value_of("output").unwrap();
            let opsdesc = if let Some(opdescs) = matches.values_of("opdesc") {
                let paths = opdescs.map(String::from).collect::<Vec<String>>();
                load_opdescs(&paths).unwrap()
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
            }
            if let Err(err) =
                compile_program_and_write_bin::<ExternalDeepValidator>(&input, &output, &opsdesc)
            {
                eprintln!("{}", err.message);
                ::std::process::exit(1);
            }
        }
        ("pst", Some(matches)) => {
            let input = matches.value_of("input").unwrap();
            let output = matches.value_of("output").unwrap();
            let pretty = matches.is_present("pretty");
            let opsdesc = if let Some(opdescs) = matches.values_of("opdesc") {
                let paths = opdescs.map(String::from).collect::<Vec<String>>();
                load_opdescs(&paths).unwrap()
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
            }
            if let Err(err) = compile_program_and_write_pst::<ExternalDeepValidator>(
                &input, &output, &opsdesc, pretty,
            ) {
                eprintln!("{}", err.message);
                ::std::process::exit(1);
            }
        }
        _ => {
            eprintln!("{}", matches.usage());
            ::std::process::exit(1);
        }
    }
}
