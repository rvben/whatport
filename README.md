# whatport

[![CI](https://github.com/rvben/whatport/actions/workflows/ci.yml/badge.svg)](https://github.com/rvben/whatport/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/whatport.svg)](https://crates.io/crates/whatport)
[![clispec](https://img.shields.io/badge/clispec-v0.2-blue)](https://clispec.dev)

Find what process is listening on a TCP/UDP port, and free it. The fast,
agent-friendly answer to "what's on :5099?" and "kill whatever it is."

## Why

"Address already in use." Then the dance: `lsof -i :5099`, read the pid,
`kill`, hope. `whatport` collapses that into one command with structured
output: `whatport 5099` tells you the owner; `whatport kill 5099` frees it.
JSON when piped, a table in a terminal.

## Install

```sh
cargo install whatport
```

## Quickstart

```sh
# What's on this port?
whatport 5099
# PORT   PROTO PID      PROCESS                USER       UPTIME    ADDRESS
# 5099   tcp   1234     node                   ruben      3h12m     127.0.0.1

# Just the data (JSON when piped)
whatport 5099 | jq '.listeners[0].pid'

# Free it (SIGTERM, or --force for SIGKILL)
whatport kill 5099
whatport kill 5099 --force

# Everything that's listening
whatport list
whatport list --proto udp
```

`whatport <port>` is shorthand for `whatport inspect <port>`.

## Commands

| command | mutating | what it does |
| ------- | -------- | ------------ |
| `whatport <port>` / `inspect <port>` | no | the listener(s) on a port: pid, process, command, user, uptime, address |
| `list` | no | every listening TCP/UDP socket |
| `kill <port> [--force]` | **yes** | SIGTERM (or SIGKILL with `--force`) the owner(s); reports what it signalled |
| `schema` | no | the clispec v0.2 contract as JSON |
| `completions <shell>` | no | shell completion script |

Global flags: `-o`/`--output auto\|json\|text` (auto = text on a TTY, JSON when
piped) and `--proto tcp\|udp\|all`.

## Output

- **Inspect/list:** `{"listeners":[{port, proto, addr, pid, process, command, user, uptime_secs}, …]}`.
- **Kill:** `{"port", "signal", "killed":[{pid, process, ok, …}], "changed"}`.

When the OS won't let you see the owner of a socket you don't own, the port and
protocol are still shown and `pid`/`process` are omitted - run with `sudo` for
full owner details rather than getting a silent gap.

## Platforms

macOS and Linux. Pure Rust (`netstat2` for the socket table, `sysinfo` for
process details) - no shelling out to `lsof`/`ss`.

## Exit codes

| code | meaning |
| ---- | ------- |
| `0`  | success |
| `1`  | nothing listening on the queried port |
| `2`  | system error (socket/process table unreadable, or a kill failed) |
| `3`  | usage error (bad arguments) |

## For agents (clispec)

whatport follows [The CLI Spec](https://clispec.dev): structured output on
stdout, structured error envelopes on the last line of stderr, and a `schema`
subcommand whose output validates against `clispec.dev/schema/v0.2.json`
(checked by the test suite). Read-only commands are `mutating: false`; `kill`
is the one `mutating: true` command.

## License

MIT
