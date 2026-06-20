//! whatport: find what process is listening on a TCP/UDP port, and free it.
//!
//! The whole pipeline is reachable through [`run`], which the CLI and the tests
//! both use. `run` is generic over [`Probe`] (the socket/process source) and
//! [`Killer`] (the signaller) so tests drive it with fakes - no real sockets or
//! processes touched.

mod error;
mod kill;
mod model;
mod output;
mod probe;
pub mod schema;

pub use error::WhatportError;
pub use kill::{Killer, Signal, SystemKiller};
pub use model::{Listener, Proto, ProtoFilter};
pub use probe::{Probe, SystemProbe};

use serde::Serialize;
use std::collections::BTreeSet;

/// Rendered output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

/// What to look at: one port, or every listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    Port(u16),
    All,
}

/// What to do: just report, or signal the owner(s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Inspect,
    Kill(Signal),
}

/// A complete whatport request.
#[derive(Debug, Clone)]
pub struct Request {
    pub query: Query,
    pub proto: ProtoFilter,
    pub action: Action,
    pub format: OutputFormat,
}

/// The outcome of signalling one pid (one row of the kill summary).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KillResult {
    pub pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    pub signal: &'static str,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run a whatport request and return the rendered output (no trailing newline).
pub fn run<P: Probe, K: Killer>(
    probe: &P,
    killer: &K,
    req: &Request,
) -> Result<String, WhatportError> {
    let matched: Vec<Listener> = probe
        .listeners()?
        .into_iter()
        .filter(|l| req.proto.matches(l.proto))
        .filter(|l| match req.query {
            Query::Port(p) => l.port == p,
            Query::All => true,
        })
        .collect();

    match req.action {
        Action::Inspect => {
            if let Query::Port(p) = req.query
                && matched.is_empty()
            {
                return Err(WhatportError::NoListener {
                    port: p,
                    proto: req.proto.label(),
                });
            }
            Ok(output::render_listeners(&matched, req.format))
        }
        Action::Kill(signal) => {
            let Query::Port(port) = req.query else {
                return Err(WhatportError::Usage {
                    message: "kill requires a specific port".into(),
                });
            };
            if matched.is_empty() {
                return Err(WhatportError::NoListener {
                    port,
                    proto: req.proto.label(),
                });
            }

            let mut seen = BTreeSet::new();
            let mut results = Vec::new();
            for l in &matched {
                let Some(pid) = l.pid else { continue };
                if !seen.insert(pid) {
                    continue;
                }
                let result = match killer.kill(pid, signal) {
                    Ok(()) => KillResult {
                        pid,
                        process: l.process.clone(),
                        signal: signal.as_str(),
                        ok: true,
                        error: None,
                    },
                    Err(e) => KillResult {
                        pid,
                        process: l.process.clone(),
                        signal: signal.as_str(),
                        ok: false,
                        error: Some(e.to_string()),
                    },
                };
                results.push(result);
            }

            if results.is_empty() {
                return Err(WhatportError::System {
                    message: format!(
                        "a listener holds port {port} but its pid is not visible; try elevated privileges"
                    ),
                });
            }
            Ok(output::render_kill(port, signal, &results, req.format))
        }
    }
}
