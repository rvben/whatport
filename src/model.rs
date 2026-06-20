//! Core data model: a listening socket and the process behind it.

use serde::Serialize;

/// Transport protocol of a listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Proto {
    Tcp,
    Udp,
}

impl Proto {
    pub fn as_str(self) -> &'static str {
        match self {
            Proto::Tcp => "tcp",
            Proto::Udp => "udp",
        }
    }
}

/// A bound/listening socket plus whatever is known about the owning process.
///
/// The process fields are `Option` because the OS does not always permit a
/// caller to see the owner of a socket it does not own (a clear note is shown
/// rather than failing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listener {
    pub port: u16,
    pub proto: Proto,
    /// Local bind address (e.g. `0.0.0.0`, `127.0.0.1`, `::`).
    pub addr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_secs: Option<u64>,
}

impl Listener {
    /// Distinct owning pid, if known.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }
}

/// Which protocols to consider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtoFilter {
    Tcp,
    Udp,
    All,
}

impl ProtoFilter {
    pub fn matches(self, proto: Proto) -> bool {
        match self {
            ProtoFilter::All => true,
            ProtoFilter::Tcp => proto == Proto::Tcp,
            ProtoFilter::Udp => proto == Proto::Udp,
        }
    }

    /// The proto label for a single-protocol filter (for error messages).
    pub fn label(self) -> Option<&'static str> {
        match self {
            ProtoFilter::Tcp => Some("tcp"),
            ProtoFilter::Udp => Some("udp"),
            ProtoFilter::All => None,
        }
    }
}
