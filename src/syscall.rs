//! Defines how system call representations are parsed from a given mapping configuration, and how
//! a `SyscallManager` is instantiated to consume system call input from the confine tracer.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::SyscallError;

/*
/// Defines enum for various system call group, which classifies syscalls to groups that
/// define generalized functionality.
/// Inspired by: http://seclab.cs.sunysb.edu/sekar/papers/syscallclassif.htm
#[derive(Deserialize, Debug, Clone)]
pub enum SyscallGroup {
    FileIO,
    ProcessControl,
    NetworkAccess,
    MessageQueues,
    SharedMemory,
    TimeControl,
    Ungrouped, // .. other miscellaneous system-related tasks
}
*/

/// declares an action parsed by the userspace application and applied to
/// system calls before trace.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SyscallAction {
    Permit, // enable execution of system call
    Warn,   // warns user through STDOUT, but continue trace
    Block,  // SIGINT to trace execution when encountering call
    Log,    // log syscall execution to log
}

/// Represents a single system call definition, including its syscall number, name,
/// and a vector of argument definitions.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Syscall {
    pub number: u64,
    pub name: String,
    pub args: Vec<String>,
}

/// Represents a parsed system call from a tracer, storing only the name, and collapsing arguments
/// as a hashmap between types and parsed values from registers in the calling convention.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSyscall {
    pub name: String,
    pub args: HashMap<String, Value>,
}

/// Defines an interface for parsing and displaying system calls parsed by confine
#[derive(Serialize, Deserialize, Clone)]
pub struct SyscallManager {
    /// stores the system calls that are parsed during confine execution
    syscalls: Vec<ParsedSyscall>,

    /// stores all current state of system calls for the kernel
    #[serde(skip)]
    pub syscall_table: Vec<Syscall>,
}

impl SyscallManager {
    /// Generates syscall table to parse incoming system calls with
    pub fn new() -> Result<Self, SyscallError> {
        let syscall_table =
            SyscallManager::parse_syscall_table().map_err(|_| SyscallError::SyscallTableError {
                reason: "Cannot deserialize system calls mapping",
            })?;
        Ok(Self {
            syscalls: Vec::new(),
            syscall_table,
        })
    }

    /// Helper to parse JSON-based system call mapping to store for confine to consult when
    /// executing a trace.
    #[inline]
    pub fn parse_syscall_table() -> serde_json::Result<Vec<Syscall>> {
        // get path to syscall JSON configuration to parse with crate root
        let mut root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.push("extras/syscall_table.json");

        // read from JSON data from path
        let syscall_data: String = std::fs::read_to_string(root).unwrap();

        // deserialize as Vec of strongly typed system calls and args
        serde_json::from_str(&syscall_data)
    }

    /// Given a parsed syscall number from ORIG_RAX, get arguments for the specific system call
    /// such that tracer can appropriately read from memory addresses.
    pub fn get_arguments(&mut self, number: u64) -> Result<Vec<String>, SyscallError> {
        match self
            .syscall_table
            .iter()
            .position(|syscall| syscall.number == number)
        {
            Some(idx) => Ok(self.syscall_table[idx].args.clone()),
            None => Err(SyscallError::UnsupportedSyscall { id: number }),
        }
    }

    /// Given a syscall number and parsed arguments, instantiate a `ParsedSyscall` and add
    /// for later consumption and display.
    pub fn add_syscall(
        &mut self,
        number: u64,
        args: HashMap<String, Value>,
    ) -> Result<(), SyscallError> {
        // retrieve syscall name
        let name: String = match self
            .syscall_table
            .iter()
            .find(|syscall| syscall.number == number)
            .map(|syscall| syscall.name.clone())
        {
            Some(res) => res,
            None => {
                return Err(SyscallError::UnsupportedSyscall { id: number });
            }
        };

        // instantiate new ParsedSyscall and push
        self.syscalls.push(ParsedSyscall { name, args });
        Ok(())
    }
}
