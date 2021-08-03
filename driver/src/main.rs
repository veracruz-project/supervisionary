//! # Entry point for the driver application
//!
//! # Authors
//!
//! [Dominic Mulligan], Systems Research Group, [Arm Research] Cambridge.
//!
//! # Copyright
//!
//! Copyright (c) Arm Limited, 2021.  All rights reserved (r).  Please see the
//! `LICENSE.markdown` file in the *Supervisionary* root directory for licensing
//! information.
//!
//! [Dominic Mulligan]: https://dominic-mulligan.co.uk
//! [Arm Research]: http://www.arm.com/research

use anyhow::Result;
use clap::{App, Arg};
use env_logger;
use log::info;
use std::{path::PathBuf, process::exit};

////////////////////////////////////////////////////////////////////////////////
// Useful constants.
////////////////////////////////////////////////////////////////////////////////

const APPLICATION_NAME: &str = "Supervisionary driver.";
const ABOUT_MESSAGE: &str = "Main driver application for Supervisionary tests.";
const AUTHOR_LIST: &str = "The Supervisionary Development Team.";
const VERSION_NUMBER: &str = "0.1.0";

////////////////////////////////////////////////////////////////////////////////
// Command-line parsing.
////////////////////////////////////////////////////////////////////////////////

/// Captures the command line arguments passed to the program.
struct CommandLineArguments {
    /// The path of the Wasm binary to load.
    wasm_binary_path: PathBuf,
}

/// Parses the command line arguments of the program, exiting with an error code
/// if this cannot be done successfully.  Otherwise, packs the command line
/// arguments into a `CommandLineArguments` value, which is returned.
fn parse_command_line_arguments() -> CommandLineArguments {
    let mut app = App::new(APPLICATION_NAME)
        .about(ABOUT_MESSAGE)
        .version(VERSION_NUMBER)
        .author(AUTHOR_LIST);

    app.arg(
        Arg::new("wasm-binary-path")
            .required(true)
            .short('b')
            .long("binary")
            .about("Path to the Wasm binary to load"),
    );

    let matches = app.get_matches();

    if let Some(path) = matches.value_of("wasm-binary-path") {
        CommandLineArguments {
            wasm_binary_path: PathBuf::from(path),
        }
    } else {
        eprintln!("No Wasm binary path provided as argument.");
        exit(1)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Entry point.
////////////////////////////////////////////////////////////////////////////////

fn main() {
    env_logger::init();

    let command_line_args = parse_command_line_arguments();
}
