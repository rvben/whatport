//! Real-system tests: exercise `SystemProbe` and `SystemKiller` against live
//! sockets and a live process - no fakes. These complement the fake-driven
//! logic tests in behavior.rs by proving the OS-facing paths actually work.
//!
//! The kill round-trip needs `python3` (present on CI runners and dev macs);
//! it skips cleanly if absent rather than failing spuriously.

use std::io::{BufRead, BufReader};
use std::net::{TcpListener, UdpSocket};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use whatport::{Probe, Proto, SystemProbe};

const BIN: &str = env!("CARGO_BIN_EXE_whatport");

#[test]
fn system_probe_finds_a_real_udp_socket() {
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = sock.local_addr().unwrap().port();

    let found = SystemProbe
        .listeners()
        .unwrap()
        .into_iter()
        .find(|l| l.port == port && l.proto == Proto::Udp)
        .expect("SystemProbe should find the bound UDP socket");
    assert_eq!(
        found.pid,
        Some(std::process::id()),
        "UDP owner pid should be us"
    );
}

#[test]
fn system_probe_finds_an_ipv6_listener() {
    let sock = match TcpListener::bind("[::1]:0") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("skipping: no IPv6 loopback on this host");
            return;
        }
    };
    let port = sock.local_addr().unwrap().port();

    let found = SystemProbe
        .listeners()
        .unwrap()
        .into_iter()
        .any(|l| l.port == port && l.proto == Proto::Tcp);
    assert!(
        found,
        "SystemProbe should find the IPv6 TCP listener on {port}"
    );
}

#[test]
fn kill_terminates_a_real_process_holding_a_port() {
    // A child that binds a TCP port, starts listening, prints the port, and waits.
    let script = "import socket,time\n\
                  s=socket.socket()\n\
                  s.bind(('127.0.0.1',0))\n\
                  s.listen()\n\
                  print(s.getsockname()[1],flush=True)\n\
                  time.sleep(60)";
    let mut child = match Command::new("python3")
        .args(["-c", script])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            eprintln!("skipping: python3 not available");
            return;
        }
    };

    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port: u16 = line.trim().parse().expect("child prints its port");
    let child_pid = child.id();

    // Kill via the real binary => real SystemKiller, end to end.
    let out = Command::new(BIN)
        .args(["kill", &port.to_string()])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(v["changed"], true);
    assert_eq!(v["killed"][0]["pid"], child_pid);
    assert_eq!(v["killed"][0]["ok"], true);

    // The child must actually have terminated.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait().unwrap() {
            Some(_) => break,
            None if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            None => {
                let _ = child.kill();
                panic!("child survived `whatport kill`");
            }
        }
    }
}
