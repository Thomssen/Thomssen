use std::net::{IpAddr, Ipv4Addr};

use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};

use crate::error::{AppError, AppResult};
use crate::netutil::ip_matches;
use crate::store::models::{ConnectionProtocol, ConnectionStatus, LiveConnection};

use super::processes::ProcessLookup;

/// IPv4 only for now - the spec's examples are all IPv4 and dual-stack
/// CIDR handling (allowlist, detection) is meaningfully more surface area.
/// A real IPv6 connection is simply left out of the list rather than
/// mis-rendered.
fn to_v4(addr: IpAddr) -> Option<Ipv4Addr> {
    match addr {
        IpAddr::V4(v4) => Some(v4),
        IpAddr::V6(_) => None,
    }
}

fn map_state(state: TcpState) -> ConnectionStatus {
    match state {
        TcpState::Closed => ConnectionStatus::Closed,
        TcpState::Listen => ConnectionStatus::Listening,
        TcpState::SynSent => ConnectionStatus::SynSent,
        TcpState::SynReceived => ConnectionStatus::SynReceived,
        TcpState::Established => ConnectionStatus::Established,
        TcpState::FinWait1 => ConnectionStatus::FinWait1,
        TcpState::FinWait2 => ConnectionStatus::FinWait2,
        TcpState::CloseWait => ConnectionStatus::CloseWait,
        TcpState::Closing => ConnectionStatus::Closing,
        TcpState::LastAck => ConnectionStatus::LastAck,
        TcpState::TimeWait => ConnectionStatus::TimeWait,
        TcpState::DeleteTcb => ConnectionStatus::DeleteTcb,
        TcpState::Unknown => ConnectionStatus::Unknown,
    }
}

/// A fresh read of every active TCP/UDP socket on this PC, cross-referenced
/// with the process table for an application name, and flagged against the
/// allowlist and the detection engine's current suspicious-IP set.
pub fn snapshot(allowlist: &[String], suspicious_ips: &std::collections::HashSet<Ipv4Addr>) -> AppResult<Vec<LiveConnection>> {
    let af_flags = AddressFamilyFlags::IPV4;
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;
    let sockets = netstat2::get_sockets_info(af_flags, proto_flags)
        .map_err(|e| AppError::new(format!("Couldn't read active connections: {e}")))?;

    let mut lookup = ProcessLookup::new();
    lookup.refresh();
    let processes = lookup.snapshot();

    let mut connections = Vec::with_capacity(sockets.len());
    for socket in sockets {
        let pid = socket.associated_pids.first().copied().unwrap_or(0);
        let process = processes.get(&pid);
        let app_name = process.map(|p| p.name.clone()).unwrap_or_else(|| "Unknown".to_string());
        let process_path = process.and_then(|p| p.exe_path.clone());

        let (local_v4, remote_v4, local_port, remote_port, protocol, status) = match &socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) => {
                let Some(local) = to_v4(tcp.local_addr) else { continue };
                let Some(remote) = to_v4(tcp.remote_addr) else { continue };
                (local, remote, tcp.local_port, tcp.remote_port, ConnectionProtocol::Tcp, map_state(tcp.state))
            }
            ProtocolSocketInfo::Udp(udp) => {
                let Some(local) = to_v4(udp.local_addr) else { continue };
                // UDP is connectionless - the OS never reports a peer for a
                // merely-bound socket. Represented as 0.0.0.0:0 rather than
                // fabricating a remote endpoint.
                (local, Ipv4Addr::UNSPECIFIED, udp.local_port, 0u16, ConnectionProtocol::Udp, ConnectionStatus::Unknown)
            }
        };

        let is_allowlisted = allowlist.iter().any(|entry| ip_matches(remote_v4, entry));
        let is_suspicious = suspicious_ips.contains(&remote_v4);

        connections.push(LiveConnection {
            id: format!("{pid}-{local_v4}-{local_port}-{remote_v4}-{remote_port}-{protocol:?}"),
            app_name,
            process_id: pid,
            process_path,
            local_address: local_v4.to_string(),
            local_port,
            remote_address: remote_v4.to_string(),
            remote_port,
            protocol,
            status,
            is_allowlisted,
            is_suspicious,
        });
    }

    Ok(connections)
}
