//! The clispec v0.3 candidate contract emitted by `whatport schema`.
//!
//! Conforms to <https://clispec.dev/schema/v0.3.json> (validated by a test
//! against the vendored copy in `schemas/clispec-v0.3.json`).

use serde_json::{Value, json};

/// The version of The CLI Spec this document conforms to.
pub const CLISPEC_VERSION: &str = "0.3";

/// Build the clispec contract as a JSON value.
pub fn contract() -> Value {
    json!({
        "clispec": CLISPEC_VERSION,
        "name": "whatport",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "output": {"tty": "text", "piped": "json"},
        "global_args": [
            {
                "name": "--output",
                "short": "-o",
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
                "effects": "read_only",
                "mutating": false,
                "cardinality": "unbounded",
                "pagination": {"style": "offset", "limit_arg": "--limit", "offset_arg": "--offset"},
                "fields_arg": "--fields",
                "stability": "stable",
                "args": [
                    {"name": "--limit", "type": "integer", "default": 100, "description": "Maximum listeners to return."},
                    {"name": "--offset", "type": "integer", "default": 0, "description": "Number of listeners to skip."},
                    {"name": "--fields", "type": "string", "description": "Comma-separated listener fields to include."}
                ],
                "example": {"args": ["list"]},
                "output_fields": listener_fields()
            },
            {
                "name": "inspect",
                "description": "Show the listener(s) on a port. Also the default command, invoked as `whatport <port>`.",
                "effects": "read_only",
                "mutating": false,
                "cardinality": "bounded",
                "stability": "stable",
                "args": [
                    {"name": "port", "type": "integer", "required": true, "description": "TCP/UDP port number (1-65535)."}
                ],
                "output_fields": listener_fields()
            },
            {
                "name": "kill",
                "description": "Signal the process(es) holding a port (SIGTERM by default).",
                "effects": "non_idempotent",
                "mutating": true,
                "cardinality": "single",
                "stability": "stable",
                "args": [
                    {"name": "port", "type": "integer", "required": true, "description": "Port whose owner(s) to signal."},
                    {"name": "--force", "type": "boolean", "required": false, "default": false, "description": "Send SIGKILL instead of SIGTERM."}
                ],
                "output_fields": [
                    {"name": "port", "type": "integer", "description": "The port that was freed."},
                    {"name": "signal", "type": "string", "description": "The signal sent (TERM or KILL)."},
                    {"name": "killed", "type": "array", "items": {"type": "object"}, "description": "Per-pid results: {pid, process, signal, ok, error}."},
                    {"name": "changed", "type": "boolean", "description": "Whether anything was actually signalled."}
                ]
            },
            {
                "name": "schema",
                "description": "Print this clispec contract as JSON.",
                "effects": "read_only",
                "mutating": false,
                "cardinality": "single",
                "stdout_schema": {"$ref": "https://clispec.dev/schema/v0.3.json"},
                "stability": "stable"
            },
            {
                "name": "completions",
                "description": "Generate a shell completion script.",
                "effects": "read_only",
                "mutating": false,
                "output_kind": "opaque",
                "media_type": "text/plain",
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
        {"name": "pid", "type": "integer", "description": "Owning pid; omitted if not permitted to see it."},
        {"name": "process", "type": "string"},
        {"name": "command", "type": "string"},
        {"name": "user", "type": "string"},
        {"name": "uptime_secs", "type": "integer"}
    ])
}

/// The contract as a pretty-printed JSON string.
pub fn contract_json() -> String {
    serde_json::to_string_pretty(&contract()).expect("contract serializes")
}
