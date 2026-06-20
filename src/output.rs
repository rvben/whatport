//! Rendering listeners and kill summaries as a table (TTY) or JSON (piped).
//!
//! Object keys in JSON are stable; the table is bounded to one row per match.

use crate::kill::Signal;
use crate::model::Listener;
use crate::{KillResult, OutputFormat};
use serde_json::json;

/// Render the matched listeners.
pub fn render_listeners(listeners: &[Listener], format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => json!({ "listeners": listeners }).to_string(),
        OutputFormat::Table => listener_table(listeners),
    }
}

/// Render the result of a kill.
pub fn render_kill(
    port: u16,
    signal: Signal,
    results: &[KillResult],
    format: OutputFormat,
) -> String {
    match format {
        OutputFormat::Json => json!({
            "port": port,
            "signal": signal.as_str(),
            "killed": results,
            "changed": results.iter().any(|r| r.ok),
        })
        .to_string(),
        OutputFormat::Table => results
            .iter()
            .map(|r| {
                let verb = if r.ok { "killed" } else { "failed to kill" };
                let proc = r.process.as_deref().unwrap_or("?");
                match &r.error {
                    Some(e) => format!("{verb} pid {} ({proc}) with SIG{}: {e}", r.pid, r.signal),
                    None => format!("{verb} pid {} ({proc}) with SIG{}", r.pid, r.signal),
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn listener_table(listeners: &[Listener]) -> String {
    if listeners.is_empty() {
        return "no listeners".to_string();
    }
    let header = format!(
        "{:<6} {:<5} {:<8} {:<22} {:<10} {:<9} {}",
        "PORT", "PROTO", "PID", "PROCESS", "USER", "UPTIME", "ADDRESS"
    );
    let mut rows = vec![header];
    for l in listeners {
        rows.push(format!(
            "{:<6} {:<5} {:<8} {:<22} {:<10} {:<9} {}",
            l.port,
            l.proto.as_str(),
            l.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
            truncate(l.process.as_deref().unwrap_or("-"), 22),
            truncate(l.user.as_deref().unwrap_or("-"), 10),
            l.uptime_secs.map(humanize).unwrap_or_else(|| "-".into()),
            l.addr,
        ));
    }
    rows.join("\n")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let keep: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{keep}…")
    }
}

fn humanize(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3_600;
    let m = (secs % 3_600) / 60;
    let s = secs % 60;
    if d > 0 {
        format!("{d}d{h}h")
    } else if h > 0 {
        format!("{h}h{m}m")
    } else if m > 0 {
        format!("{m}m{s}s")
    } else {
        format!("{s}s")
    }
}
