//! Behavior tests for whatport, exercised through the public `run` API with a
//! fake probe and killer (no real sockets or processes touched).

use std::cell::RefCell;

use whatport::{
    Action, Killer, Listener, OutputFormat, Proto, ProtoFilter, Query, Request, Signal,
    SystemProbe, WhatportError, run,
};

struct FakeProbe(Vec<Listener>);
impl whatport::Probe for FakeProbe {
    fn listeners(&self) -> Result<Vec<Listener>, WhatportError> {
        Ok(self.0.clone())
    }
}

#[derive(Default)]
struct FakeKiller {
    calls: RefCell<Vec<(u32, Signal)>>,
    fail: Vec<u32>,
}
impl Killer for FakeKiller {
    fn kill(&self, pid: u32, signal: Signal) -> Result<(), WhatportError> {
        self.calls.borrow_mut().push((pid, signal));
        if self.fail.contains(&pid) {
            Err(WhatportError::KillFailed {
                pid,
                message: "boom".into(),
            })
        } else {
            Ok(())
        }
    }
}

/// A listener with sensible defaults.
fn listener(port: u16, proto: Proto, pid: Option<u32>, process: &str) -> Listener {
    Listener {
        port,
        proto,
        addr: "127.0.0.1".into(),
        pid,
        process: Some(process.into()),
        command: None,
        user: Some("ruben".into()),
        uptime_secs: Some(5),
    }
}

fn req(query: Query, proto: ProtoFilter, action: Action, format: OutputFormat) -> Request {
    Request {
        query,
        proto,
        action,
        format,
    }
}

fn inspect(query: Query) -> Request {
    req(query, ProtoFilter::All, Action::Inspect, OutputFormat::Json)
}

fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).expect("output is JSON")
}

// ---- inspect ------------------------------------------------------------

#[test]
fn inspects_a_port() {
    let probe = FakeProbe(vec![
        listener(5099, Proto::Tcp, Some(1234), "node"),
        listener(8080, Proto::Tcp, Some(99), "caddy"),
    ]);
    let out = run(&probe, &FakeKiller::default(), &inspect(Query::Port(5099))).unwrap();
    let v = json(&out);
    let items = v["listeners"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["port"], 5099);
    assert_eq!(items[0]["pid"], 1234);
    assert_eq!(items[0]["process"], "node");
}

#[test]
fn missing_port_is_no_listener_exit_1() {
    let probe = FakeProbe(vec![listener(8080, Proto::Tcp, Some(99), "caddy")]);
    let err = run(&probe, &FakeKiller::default(), &inspect(Query::Port(5099))).unwrap_err();
    assert!(matches!(err, WhatportError::NoListener { port: 5099, .. }));
    assert_eq!(err.exit_code(), 1);
}

#[test]
fn list_returns_all_listeners() {
    let probe = FakeProbe(vec![
        listener(5099, Proto::Tcp, Some(1), "a"),
        listener(53, Proto::Udp, Some(2), "b"),
    ]);
    let out = run(&probe, &FakeKiller::default(), &inspect(Query::All)).unwrap();
    assert_eq!(json(&out)["listeners"].as_array().unwrap().len(), 2);
}

#[test]
fn proto_filter_excludes_other_protocols() {
    let probe = FakeProbe(vec![
        listener(5099, Proto::Tcp, Some(1), "a"),
        listener(5099, Proto::Udp, Some(2), "b"),
    ]);
    let r = req(
        Query::Port(5099),
        ProtoFilter::Tcp,
        Action::Inspect,
        OutputFormat::Json,
    );
    let items = json(&run(&probe, &FakeKiller::default(), &r).unwrap())["listeners"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(items, 1);
}

// ---- kill ---------------------------------------------------------------

#[test]
fn kill_signals_the_owner_with_sigterm() {
    let probe = FakeProbe(vec![listener(5099, Proto::Tcp, Some(1234), "node")]);
    let killer = FakeKiller::default();
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Term),
        OutputFormat::Json,
    );
    let out = run(&probe, &killer, &r).unwrap();
    assert_eq!(*killer.calls.borrow(), vec![(1234, Signal::Term)]);
    let v = json(&out);
    assert_eq!(v["changed"], true);
    assert_eq!(v["killed"][0]["pid"], 1234);
    assert_eq!(v["killed"][0]["ok"], true);
    assert_eq!(v["signal"], "TERM");
}

#[test]
fn kill_force_uses_sigkill() {
    let probe = FakeProbe(vec![listener(5099, Proto::Tcp, Some(1234), "node")]);
    let killer = FakeKiller::default();
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Kill),
        OutputFormat::Json,
    );
    run(&probe, &killer, &r).unwrap();
    assert_eq!(*killer.calls.borrow(), vec![(1234, Signal::Kill)]);
}

#[test]
fn kill_dedupes_pids() {
    // Same process listening on the port over IPv4 and IPv6 -> one signal.
    let probe = FakeProbe(vec![
        listener(5099, Proto::Tcp, Some(1234), "node"),
        listener(5099, Proto::Tcp, Some(1234), "node"),
    ]);
    let killer = FakeKiller::default();
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Term),
        OutputFormat::Json,
    );
    run(&probe, &killer, &r).unwrap();
    assert_eq!(killer.calls.borrow().len(), 1);
}

#[test]
fn kill_reports_failure_per_pid() {
    let probe = FakeProbe(vec![listener(5099, Proto::Tcp, Some(1234), "node")]);
    let killer = FakeKiller {
        fail: vec![1234],
        ..Default::default()
    };
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Term),
        OutputFormat::Json,
    );
    let v = json(&run(&probe, &killer, &r).unwrap());
    assert_eq!(v["killed"][0]["ok"], false);
    assert_eq!(v["changed"], false);
}

#[test]
fn kill_with_no_listener_errors() {
    let probe = FakeProbe(vec![]);
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Term),
        OutputFormat::Json,
    );
    let err = run(&probe, &FakeKiller::default(), &r).unwrap_err();
    assert!(matches!(err, WhatportError::NoListener { .. }));
}

#[test]
fn kill_without_visible_pid_is_a_system_error() {
    let probe = FakeProbe(vec![listener(5099, Proto::Tcp, None, "root-owned")]);
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Kill(Signal::Term),
        OutputFormat::Json,
    );
    let err = run(&probe, &FakeKiller::default(), &r).unwrap_err();
    assert!(matches!(err, WhatportError::System { .. }));
    assert_eq!(err.exit_code(), 2);
}

// ---- output -------------------------------------------------------------

#[test]
fn text_output_is_a_table_not_json() {
    let probe = FakeProbe(vec![listener(5099, Proto::Tcp, Some(1234), "node")]);
    let r = req(
        Query::Port(5099),
        ProtoFilter::All,
        Action::Inspect,
        OutputFormat::Table,
    );
    let out = run(&probe, &FakeKiller::default(), &r).unwrap();
    assert!(out.contains("PORT") && out.contains("5099") && out.contains("node"));
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_err());
}

// ---- real probe (integration) ------------------------------------------

#[test]
fn system_probe_finds_a_real_listener() {
    use std::net::TcpListener;
    let sock = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = sock.local_addr().unwrap().port();

    let r = req(
        Query::Port(port),
        ProtoFilter::Tcp,
        Action::Inspect,
        OutputFormat::Json,
    );
    let out = run(&SystemProbe, &FakeKiller::default(), &r).unwrap();
    let v = json(&out);
    let items = v["listeners"].as_array().unwrap();
    assert!(!items.is_empty(), "should find the bound port");
    assert_eq!(items[0]["port"], port);
    // This test process owns the socket, so the pid must be visible and be us.
    assert_eq!(items[0]["pid"], std::process::id());
}
