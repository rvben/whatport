//! Signalling the process behind a port.
//!
//! [`Killer`] is the seam: the real [`SystemKiller`] sends a signal via
//! `sysinfo`; tests substitute a fake that records calls instead of killing.

use crate::error::WhatportError;

/// The signal to send when freeing a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Graceful termination (SIGTERM). The default.
    Term,
    /// Forceful kill (SIGKILL).
    Kill,
}

impl Signal {
    pub fn as_str(self) -> &'static str {
        match self {
            Signal::Term => "TERM",
            Signal::Kill => "KILL",
        }
    }
}

/// Sends a signal to a process by pid.
pub trait Killer {
    fn kill(&self, pid: u32, signal: Signal) -> Result<(), WhatportError>;
}

/// The real killer, backed by `sysinfo`.
pub struct SystemKiller;

impl Killer for SystemKiller {
    fn kill(&self, pid: u32, signal: Signal) -> Result<(), WhatportError> {
        use sysinfo::{Pid, ProcessesToUpdate, System};

        let target = Pid::from_u32(pid);
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);

        // Never let an ambiguous socket-to-pid mapping terminate whatport or
        // the shell/test runner that launched it. Walk the current process's
        // ancestor chain before sending a signal.
        let mut protected = Some(Pid::from_u32(std::process::id()));
        while let Some(current) = protected {
            if target == current {
                return Err(WhatportError::KillFailed {
                    pid,
                    message: "refusing to signal whatport or one of its parent processes".into(),
                });
            }
            protected = sys.process(current).and_then(|process| process.parent());
        }
        let proc = sys
            .process(target)
            .ok_or_else(|| WhatportError::KillFailed {
                pid,
                message: "process not found".into(),
            })?;

        let sig = match signal {
            Signal::Term => sysinfo::Signal::Term,
            Signal::Kill => sysinfo::Signal::Kill,
        };
        match proc.kill_with(sig) {
            Some(true) => Ok(()),
            Some(false) => Err(WhatportError::KillFailed {
                pid,
                message: "the OS rejected the signal".into(),
            }),
            // Signal not supported on this platform: fall back to SIGKILL.
            None if proc.kill() => Ok(()),
            None => Err(WhatportError::KillFailed {
                pid,
                message: "signal not supported on this platform".into(),
            }),
        }
    }
}
