//! The clispec v0.2 contract emitted by `whatport schema`.
//!
//! Conforms to <https://clispec.dev/schema/v0.2.json> (validated by a test
//! against the vendored copy in `schemas/clispec-v0.2.json`).

use serde_json::{Value, json};

/// The version of The CLI Spec this document conforms to.
pub const CLISPEC_VERSION: &str = "0.2";

/// Build the clispec contract as a JSON value.
pub fn contract() -> Value {
    json!({
        "clispec": CLISPEC_VERSION,
        "name": "whatport",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "global_args": [
            {
                "name": "--output",
                "type": "string",
                "enum": ["auto", "json", "text"],
                "default": "auto",
                "description": "Output format. auto = text on a TTY, JSON when piped."
            },
            {
                "name": "--proto",
                "type": "string",
                "enum": ["tcp", "udp", "all"],
                "default": "all",
                "description": "Restrict to a transport protocol."
            }
        ],
        "commands": [
            {
                "name": "list",
                "description": "List every listening TCP/UDP socket. The default when no port and no subcommand is given.",
                "mutating": false,
                "stability": "stable",
                "output_fields": listener_fields()
            },
            {
                "name": "inspect",
                "description": "Show the listener(s) on a port. Also the default command, invoked as `whatport <port>`.",
                "mutating": false,
                "stability": "stable",
                "args": [
                    {"name": "port", "type": "integer", "required": true, "description": "TCP/UDP port number (1-65535)."}
                ],
                "output_fields": listener_fields()
            },
            {
                "name": "kill",
                "description": "Signal the process(es) holding a port (SIGTERM by default).",
                "mutating": true,
                "stability": "stable",
                "args": [
                    {"name": "port", "type": "integer", "required": true, "description": "Port whose owner(s) to signal."},
                    {"name": "--force", "type": "boolean", "required": false, "default": false, "description": "Send SIGKILL instead of SIGTERM."}
                ],
                "output_fields": [
                    {"name": "port", "type": "integer", "description": "The port that was freed."},
                    {"name": "signal", "type": "string", "description": "The signal sent (TERM or KILL)."},
                    {"name": "killed", "type": "array", "description": "Per-pid results: {pid, process, signal, ok, error}."},
                    {"name": "changed", "type": "boolean", "description": "Whether anything was actually signalled."}
                ]
            },
            {
                "name": "schema",
                "description": "Print this clispec contract as JSON.",
                "mutating": false,
                "stability": "stable"
            },
            {
                "name": "completions",
                "description": "Generate a shell completion script.",
                "mutating": false,
                "stability": "stable",
                "args": [
                    {"name": "shell", "type": "string", "required": true, "enum": ["bash", "zsh", "fish", "powershell", "elvish"], "description": "Target shell."}
                ]
            }
        ],
        "errors": [
            {"kind": "usage", "exit_code": 3, "retryable": false, "description": "Invalid command-line arguments."},
            {"kind": "no_listener", "exit_code": 1, "retryable": false, "description": "Nothing is listening on the queried port."},
            {"kind": "system", "exit_code": 2, "retryable": false, "description": "The OS socket/process table could not be read, or a pid is not visible."},
            {"kind": "kill_failed", "exit_code": 2, "retryable": true, "description": "A process could not be signalled (may have exited, or needs elevated privileges)."}
        ]
    })
}

fn listener_fields() -> Value {
    json!([
        {"name": "port", "type": "integer"},
        {"name": "proto", "type": "string", "description": "tcp or udp"},
        {"name": "addr", "type": "string", "description": "Local bind address."},
        {"name": "pid", "type": "integer | null", "description": "Owning pid, null if not permitted to see it."},
        {"name": "process", "type": "string | null"},
        {"name": "command", "type": "string | null"},
        {"name": "user", "type": "string | null"},
        {"name": "uptime_secs", "type": "integer | null"}
    ])
}

/// The contract as a pretty-printed JSON string.
pub fn contract_json() -> String {
    serde_json::to_string_pretty(&contract()).expect("contract serializes")
}
