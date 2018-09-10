// Copyright 2018 Peter Williams <peter@newton.cx>
// Licensed under the MIT License.

/// A tiny tool to save the current process environment to disk,
/// and recover it later.

// TODO all over here: avoid conversions to Cow<str>?

extern crate bincode;
extern crate failure;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate shell_escape;
#[macro_use] extern crate structopt;

use bincode::{deserialize_from, serialize_into};
use failure::{Error};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use structopt::StructOpt;


#[derive(Debug, StructOpt)]
#[structopt(name = "serenv", about = "Save and restore the shell environment.")]
enum SerenvCli {
    #[structopt(name = "emit-cmd")]
    /// Emit "cmd"-format commands to restore a saved environment.
    EmitCmd(SerenvEmitCmdOptions),

    #[structopt(name = "emit-sh")]
    /// Emit "sh"-format commands to restore a saved environment.
    EmitSh(SerenvEmitShOptions),

    #[structopt(name = "save")]
    /// Save the current environment.
    Save(SerenvSaveOptions),
}

impl SerenvCli {
    fn cli(self) -> Result<(), Error> {
        match self {
            SerenvCli::EmitCmd(opts) => opts.cli(),
            SerenvCli::EmitSh(opts) => opts.cli(),
            SerenvCli::Save(opts) => opts.cli(),
        }
    }
}

fn main() -> Result<(), Error> {
    let program = SerenvCli::from_args();
    program.cli()
}


#[derive(Debug, StructOpt)]
struct SerenvEmitCmdOptions {
}

impl SerenvEmitCmdOptions {
    fn cli(self) -> Result<(), Error> {
        let f = File::open(".serenv.dat")?;
        let env: SavedEnvironment = deserialize_from(f)?;
        let mut em = EmitCmdChanges {};
        env.emit_changes(&mut em);
        Ok(())
    }
}

struct EmitCmdChanges {}

impl EmitChanges for EmitCmdChanges {
    fn emit_unset(&mut self, key: &OsStr) {
        println!("set {}=", shell_escape::windows::escape(key.to_string_lossy()));
    }

    fn emit_assign(&mut self, key: &OsStr, value: &OsStr) {
        let s = format!("{}={}", key.to_string_lossy(), value.to_string_lossy());
        println!("set {}", shell_escape::windows::escape(Cow::Borrowed(&s)));
    }
}


#[derive(Debug, StructOpt)]
struct SerenvEmitShOptions {
}

impl SerenvEmitShOptions {
    fn cli(self) -> Result<(), Error> {
        let f = File::open(".serenv.dat")?;
        let env: SavedEnvironment = deserialize_from(f)?;
        let mut em = EmitShChanges {};
        env.emit_changes(&mut em);
        Ok(())
    }
}

struct EmitShChanges {}

impl EmitChanges for EmitShChanges {
    fn emit_unset(&mut self, key: &OsStr) {
        println!("unset {};", shell_escape::unix::escape(key.to_string_lossy()));
    }

    fn emit_assign(&mut self, key: &OsStr, value: &OsStr) {
        println!("export {}={};",
                 shell_escape::unix::escape(key.to_string_lossy()),
                 shell_escape::unix::escape(value.to_string_lossy()));
    }
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

    pub fn emit_changes<T: EmitChanges>(&self, emitter: &mut T) {
        let mut handled = HashSet::new();

        for (os_key, os_value) in env::vars_os() {
            match self.env.get(&os_key) {
                None => {
                    emitter.emit_unset(&os_key);
                },

                Some(saved_value) => {
                    if &os_value == saved_value {
                        // Nothing to do.
                    } else {
                        emitter.emit_assign(&os_key, saved_value);
                    }

                    handled.insert(os_key);
                }
            }
        }

        for (os_key, os_value) in &self.env {
            if handled.contains(os_key) {
                continue;
            }

            emitter.emit_assign(os_key, os_value);
        }
    }
}


/// A helper trait for making emission pluggible

trait EmitChanges {
    fn emit_unset(&mut self, key: &OsStr);
    fn emit_assign(&mut self, key: &OsStr, value: &OsStr);
}
