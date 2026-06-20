//! Error type, the stable error `kind` set, and the exit-code contract.
//!
//! Errors are reported as a clispec structured envelope on the last line of
//! stderr: `{"error":{"kind":...,"message":...,"exit_code":...,"hint":...}}`.
//!
//! Exit codes (also declared in the schema):
//! - `1` no listener on the queried port
//! - `2` a system probe or kill failure
//! - `3` usage error (bad arguments)

use thiserror::Error;

/// All failure modes of a whatport run.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WhatportError {
    /// Invalid command-line arguments (also used for wrapped clap errors).
    #[error("{message}")]
    Usage { message: String },

    /// Nothing is listening on the queried port.
    #[error("nothing is listening on port {port}{}", proto_suffix(.proto))]
    NoListener {
        port: u16,
        proto: Option<&'static str>,
    },

    /// The OS socket/process tables could not be read.
    #[error("could not read system socket table: {message}")]
    System { message: String },

    /// A process could not be signalled.
    #[error("could not signal pid {pid}: {message}")]
    KillFailed { pid: u32, message: String },
}

fn proto_suffix(proto: &Option<&'static str>) -> String {
    match proto {
        Some(p) => format!("/{p}"),
        None => String::new(),
    }
}

impl WhatportError {
    /// Stable snake_case identifier consumers branch on (the schema `errors` set).
    pub fn kind(&self) -> &'static str {
        match self {
            WhatportError::Usage { .. } => "usage",
            WhatportError::NoListener { .. } => "no_listener",
            WhatportError::System { .. } => "system",
            WhatportError::KillFailed { .. } => "kill_failed",
        }
    }

    /// Actionable remediation, when there is one.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            WhatportError::Usage { .. } => Some("see `whatport --help` or `whatport schema`"),
            WhatportError::NoListener { .. } => Some("run `whatport list` to see all listeners"),
            WhatportError::KillFailed { .. } => {
                Some("the process may have already exited, or need elevated privileges")
            }
            WhatportError::System { .. } => None,
        }
    }

    /// The process exit code associated with this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            WhatportError::NoListener { .. } => 1,
            WhatportError::System { .. } | WhatportError::KillFailed { .. } => 2,
            WhatportError::Usage { .. } => 3,
        }
    }
}
