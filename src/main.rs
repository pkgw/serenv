// Copyright 2018 Peter Williams <peter@newton.cx>
// Licensed under the MIT License.

/// A tiny tool to save the current process environment to disk,
/// and recover it later.

extern crate bincode;
extern crate failure;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate shell_escape;
#[macro_use] extern crate structopt;

use bincode::{deserialize_from, serialize_into};
use failure::{Error};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs::File;
use structopt::StructOpt;


#[derive(Debug, StructOpt)]
#[structopt(name = "serenv", about = "Save and restore the shell environment.")]
enum SerenvCli {
    #[structopt(name = "save")]
    /// Save the current environment.
    Save(SerenvSaveOptions),

    #[structopt(name = "emit-sh")]
    /// Emit "sh"-format commands to restore a saved environment.
    EmitSh(SerenvEmitShOptions),
}

impl SerenvCli {
    fn cli(self) -> Result<(), Error> {
        match self {
            SerenvCli::Save(opts) => opts.cli(),
            SerenvCli::EmitSh(opts) => opts.cli(),
        }
    }
}

fn main() -> Result<(), Error> {
    let program = SerenvCli::from_args();
    program.cli()
}


#[derive(Debug, StructOpt)]
struct SerenvSaveOptions {
}

impl SerenvSaveOptions {
    fn cli(self) -> Result<(), Error> {
        let env = SavedEnvironment::from_env();
        let f = File::create(".serenv.dat")?;
        serialize_into(f, &env)?;
        Ok(())
    }
}


#[derive(Debug, StructOpt)]
struct SerenvEmitShOptions {
}

impl SerenvEmitShOptions {
    fn cli(self) -> Result<(), Error> {
        let f = File::open(".serenv.dat")?;
        let env: SavedEnvironment = deserialize_from(f)?;
        env.emit_sh();
        Ok(())
    }
}


/// The structure that (de)serializes the environment.
#[derive(Debug, Deserialize, Serialize)]
struct SavedEnvironment {
    env: HashMap<OsString, OsString>,
}

impl SavedEnvironment {
    pub fn from_env() -> Self {
        let mut env = HashMap::new();

        for (key, value) in env::vars_os() {
            env.insert(key, value);
        }

        SavedEnvironment { env }
    }

    pub fn emit_sh(&self) {
        // TODO all over here: avoid conversion to &str?
        let mut handled = HashSet::new();

        for (os_key, os_value) in env::vars_os() {
            match self.env.get(&os_key) {
                None => {
                    println!("unset {};", os_key.to_string_lossy());
                },

                Some(saved_value) => {
                    if &os_value == saved_value {
                        // Nothing to do.
                    } else {
                        println!("export {}={};", os_key.to_string_lossy(),
                                 shell_escape::unix::escape(saved_value.to_string_lossy()));
                    }

                    handled.insert(os_key);
                }
            }
        }

        for (os_key, os_value) in &self.env {
            if handled.contains(os_key) {
                continue;
            }

            println!("export {}={};", os_key.to_string_lossy(),
                     shell_escape::unix::escape(os_value.to_string_lossy()));
        }
    }
}

