//! End-to-end tests of the compiled binary: real socket lookup, the clispec
//! error envelope, and the process exit-code contract.

use std::net::TcpListener;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_whatport");

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Output {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("spawn whatport");
    Output {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// The `error` object from the last line of stderr (the clispec envelope).
fn error_envelope(stderr: &str) -> serde_json::Value {
    let last = stderr.lines().last().expect("stderr has an error line");
    serde_json::from_str::<serde_json::Value>(last).expect("error envelope is JSON")["error"]
        .clone()
}

#[test]
fn list_emits_json_when_piped() {
    let out = run(&["list"]);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
    assert!(v["listeners"].is_array());
}

#[test]
fn bare_port_finds_a_real_listener() {
    let sock = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = sock.local_addr().unwrap().port();
    let out = run(&[&port.to_string()]);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(v["listeners"][0]["port"], port);
    assert_eq!(v["listeners"][0]["pid"], std::process::id());
}

#[test]
fn schema_subcommand_is_clispec_v0_2() {
    let out = run(&["schema"]);
    assert_eq!(out.code, 0);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    assert_eq!(v["clispec"], "0.2");
    assert_eq!(v["name"], "whatport");
}

#[test]
fn help_mentions_schema_and_kill() {
    let out = run(&["--help"]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("schema"));
    assert!(out.stdout.contains("kill"));
}

#[test]
fn free_port_exits_1_with_envelope() {
    // A high port extremely unlikely to be in use.
    let out = run(&["65432"]);
    assert_eq!(out.code, 1);
    assert_eq!(error_envelope(&out.stderr)["kind"], "no_listener");
}

#[test]
fn bad_argument_exits_3_with_usage_envelope() {
    let out = run(&["--no-such-flag"]);
    assert_eq!(out.code, 3);
    assert_eq!(error_envelope(&out.stderr)["kind"], "usage");
}

#[test]
fn no_args_exits_3() {
    let out = run(&[]);
    assert_eq!(out.code, 3);
    assert_eq!(error_envelope(&out.stderr)["kind"], "usage");
}
