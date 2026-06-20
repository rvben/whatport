//! Discovering listening sockets and the processes behind them.
//!
//! [`Probe`] is the seam: the real [`SystemProbe`] reads the OS socket table
//! (`netstat2`) and process table (`sysinfo`); tests substitute a fake.

use crate::error::WhatportError;
use crate::model::{Listener, Proto};

/// Source of the current set of listening sockets.
pub trait Probe {
    /// All listening TCP sockets and bound UDP sockets, with owning-process
    /// details where the OS permits.
    fn listeners(&self) -> Result<Vec<Listener>, WhatportError>;
}

/// The real probe: `netstat2` for the socket table, `sysinfo` for processes.
pub struct SystemProbe;

impl Probe for SystemProbe {
    fn listeners(&self) -> Result<Vec<Listener>, WhatportError> {
        use netstat2::{
            AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info,
        };
        use sysinfo::{Pid, ProcessesToUpdate, System, Users};

        let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto = ProtocolFlags::TCP | ProtocolFlags::UDP;
        let sockets = get_sockets_info(af, proto).map_err(|e| WhatportError::System {
            message: e.to_string(),
        })?;

        let mut sys = System::new();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        let users = Users::new_with_refreshed_list();

        let mut out = Vec::new();
        for si in sockets {
            let (proto, addr, port) = match &si.protocol_socket_info {
                ProtocolSocketInfo::Tcp(t) => {
                    if t.state != TcpState::Listen {
                        continue;
                    }
                    (Proto::Tcp, t.local_addr.to_string(), t.local_port)
                }
                ProtocolSocketInfo::Udp(u) => (Proto::Udp, u.local_addr.to_string(), u.local_port),
            };

            let pid = si.associated_pids.first().copied();
            let proc = pid.and_then(|p| sys.process(Pid::from_u32(p)));
            let (process, command, user, uptime_secs) = match proc {
                Some(p) => {
                    let name = Some(p.name().to_string_lossy().into_owned());
                    let cmd = p.cmd();
                    let command = if cmd.is_empty() {
                        None
                    } else {
                        Some(
                            cmd.iter()
                                .map(|s| s.to_string_lossy())
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    };
                    let user = p
                        .user_id()
                        .and_then(|uid| users.get_user_by_id(uid))
                        .map(|u| u.name().to_string());
                    (name, command, user, Some(p.run_time()))
                }
                None => (None, None, None, None),
            };

            out.push(Listener {
                port,
                proto,
                addr,
                pid,
                process,
                command,
                user,
                uptime_secs,
            });
        }
        Ok(out)
    }
}
