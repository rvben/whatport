//! whatport CLI: inspect or free TCP/UDP ports.
//!
//! Follows The CLI Spec (clispec.dev): structured output on stdout (text on a
//! TTY, JSON when piped), structured error envelopes on the last line of
//! stderr, a `schema` subcommand, and non-interactive behavior. Read-only
//! commands are `mutating: false`; `kill` is the one mutating command.

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use clap::error::ErrorKind as ClapErrorKind;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use serde_json::json;
use whatport::{
    Action, OutputFormat, ProtoFilter, Query, Request, Signal, SystemKiller, SystemProbe,
    WhatportError, run, schema,
};

#[derive(Parser)]
#[command(
    name = "whatport",
    version,
    about = "Find what process is listening on a TCP/UDP port, and free it.",
    long_about = "Find what process is listening on a TCP/UDP port, and free it.\n\n\
                  `whatport <port>` shows the owner; `whatport kill <port>` frees it; \
                  `whatport list` shows everything listening.\n\n\
                  Run `whatport schema` for the machine-readable contract (clispec.dev).",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Port to inspect, e.g. `whatport 5099` (shorthand for `whatport inspect <port>`).
    #[arg(value_name = "PORT")]
    port: Option<u16>,

    /// Output format; auto = text on a TTY, JSON when piped.
    #[arg(long, short = 'o', value_enum, default_value = "auto", global = true)]
    output: CliOutput,

    /// Restrict to a transport protocol.
    #[arg(long, value_enum, default_value = "all", global = true)]
    proto: CliProto,
}

#[derive(Subcommand)]
enum Command {
    /// List every listening TCP/UDP socket.
    List,
    /// Show the listener(s) on a port.
    Inspect {
        #[arg(value_name = "PORT")]
        port: u16,
    },
    /// Signal the process(es) holding a port (SIGTERM by default).
    Kill {
        #[arg(value_name = "PORT")]
        port: u16,
        /// Send SIGKILL instead of SIGTERM.
        #[arg(long)]
        force: bool,
    },
    /// Print the machine-readable contract (clispec.dev) as JSON.
    Schema,
    /// Generate a shell completion script.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CliOutput {
    Auto,
    Json,
    Text,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliProto {
    Tcp,
    Udp,
    All,
}

impl From<CliProto> for ProtoFilter {
    fn from(p: CliProto) -> Self {
        match p {
            CliProto::Tcp => ProtoFilter::Tcp,
            CliProto::Udp => ProtoFilter::Udp,
            CliProto::All => ProtoFilter::All,
        }
    }
}

impl CliOutput {
    fn resolve(self) -> OutputFormat {
        match self {
            CliOutput::Json => OutputFormat::Json,
            CliOutput::Text => OutputFormat::Table,
            CliOutput::Auto => {
                if std::io::stdout().is_terminal() {
                    OutputFormat::Table
                } else {
                    OutputFormat::Json
                }
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return handle_clap_error(e),
    };

    let (query, action) = match &cli.command {
        Some(Command::Schema) => {
            println!("{}", schema::contract_json());
            return ExitCode::SUCCESS;
        }
        Some(Command::Completions { shell }) => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            clap_complete::generate(*shell, &mut cmd, name, &mut std::io::stdout());
            return ExitCode::SUCCESS;
        }
        Some(Command::List) => (Query::All, Action::Inspect),
        Some(Command::Inspect { port }) => (Query::Port(*port), Action::Inspect),
        Some(Command::Kill { port, force }) => (
            Query::Port(*port),
            Action::Kill(if *force { Signal::Kill } else { Signal::Term }),
        ),
        None => match cli.port {
            Some(port) => (Query::Port(port), Action::Inspect),
            None => {
                let err = WhatportError::Usage {
                    message: "no port given (try `whatport <port>` or `whatport list`)".into(),
                };
                emit_error(&err);
                return ExitCode::from(err.exit_code() as u8);
            }
        },
    };

    let request = Request {
        query,
        proto: cli.proto.into(),
        action,
        format: cli.output.resolve(),
    };

    match run(&SystemProbe, &SystemKiller, &request) {
        Ok(output) => {
            let _ = writeln!(std::io::stdout(), "{output}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            emit_error(&err);
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

/// Help and version print normally and exit 0; every other clap failure becomes
/// a structured `usage` error envelope (so a bad invocation stays parseable).
fn handle_clap_error(e: clap::Error) -> ExitCode {
    match e.kind() {
        ClapErrorKind::DisplayHelp
        | ClapErrorKind::DisplayVersion
        | ClapErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            let _ = e.print();
            ExitCode::SUCCESS
        }
        _ => {
            let err = WhatportError::Usage {
                message: e.to_string().trim().to_string(),
            };
            emit_error(&err);
            ExitCode::from(err.exit_code() as u8)
        }
    }
}

/// Write the clispec error envelope as the last line of stderr.
fn emit_error(err: &WhatportError) {
    let mut error = serde_json::Map::new();
    error.insert("kind".into(), json!(err.kind()));
    error.insert("message".into(), json!(err.to_string()));
    error.insert("exit_code".into(), json!(err.exit_code()));
    if let Some(hint) = err.hint() {
        error.insert("hint".into(), json!(hint));
    }
    eprintln!("{}", json!({ "error": error }));
}
