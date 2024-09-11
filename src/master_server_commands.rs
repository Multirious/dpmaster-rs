use std::{
    io::{self, Write},
    net::{SocketAddr, SocketAddrV4},
};
use thiserror::Error;

use crate::ParseResponseError;

use super::PREFIX;

#[derive(Debug, Error)]
#[error("Write getservers error")]
pub enum WriteHeartBeatError {
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// The heartbeat is sent by a server when it wants to get noticed by a
/// master. A server should send an heartbeat each time it becomes empty
/// or full, or stop being empty or full, plus it should make sure the
/// master gets at least one heartbeat from it every 10 or 15 minutes,
/// so the master doesn't remove it from its list of active servers.
#[doc(alias = "heartbeat")] // TODO: finish the protocol
pub fn write_heart_beat<W: Write>(
    mut w: W,
    protocol_string: &[u8],
) -> Result<u64, WriteHeartBeatError> {
    w.write_all(PREFIX)?;
    w.write_all(b"heartbeat")?;
    w.write_all(b" ")?;
    w.write_all(protocol_string)?;
    w.write_all(b"\n")?;
    let written = u64_len(PREFIX)
        + u64_len(b"heartbeat")
        + u64_len(b" ")
        + u64_len(protocol_string)
        + u64_len(b"\n");
    Ok(written)
}

#[derive(Debug, Error)]
#[error("Write getservers error")]
pub enum WriteGetServersError {
    #[error("{0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Default)]
pub struct GetServersFilter<'a> {
    pub empty: bool,
    pub full: bool,
    pub gametype: Option<&'a [u8]>,
}
impl GetServersFilter<'_> {
    pub fn write<W: Write>(&self, mut w: W) -> io::Result<u64> {
        let mut written = 0;
        if self.empty {
            w.write_all(b" empty")?;
            written += u64_len(b" empty");
        }
        if self.full {
            w.write_all(b" full")?;
            written += u64_len(b" full");
        }
        if let Some(game_type) = self.gametype {
            w.write_all(b" ")?;
            w.write_all(game_type)?;
            written += u64_len(b" ") + u64_len(game_type);
        }
        Ok(written)
    }
}

/// A "getservers" message is sent to a master by a client who wants
/// to get a list of servers. It triggers a "getserversReponse"
/// message from the master.
#[doc(alias = "getservers")]
pub fn write_get_servers<W: Write>(
    mut w: W,
    game_name: Option<&[u8]>,
    protocol_version: &[u8],
    filter: &GetServersFilter,
) -> Result<u64, WriteGetServersError> {
    let mut written = 0;
    w.write_all(PREFIX)?;
    w.write_all(b"getservers")?;
    if let Some(game_name) = game_name {
        w.write_all(b" ")?;
        w.write_all(game_name)?;
        written += u64_len(b" ") + u64_len(game_name);
    }
    w.write_all(b" ")?;
    w.write_all(protocol_version)?;
    written += filter.write(&mut w)?;
    written += u64_len(PREFIX) + u64_len(b"getservers") + u64_len(b" ") + u64_len(protocol_version);
    Ok(written)
}

#[derive(Debug, Error)]
#[error("Write getserversExt error")]
pub enum WriteGetServersExtError {
    #[error("{0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Default)]
pub struct GetServersExtFilter<'a> {
    pub empty: bool,
    pub full: bool,
    pub gametype: Option<&'a [u8]>,
    pub ipv4: bool,
    pub ipv6: bool,
}
impl GetServersExtFilter<'_> {
    pub fn write<W: Write>(&self, mut w: W) -> io::Result<u64> {
        let mut written = 0u64;
        if self.empty {
            w.write_all(b" empty")?;
            written += u64_len(b" empty");
        }
        if self.full {
            w.write_all(b" full")?;
            written += u64_len(b" full");
        }
        if let Some(game_type) = self.gametype {
            w.write_all(b" ")?;
            w.write_all(game_type)?;
            written += u64_len(b" ") + game_type.len() as u64;
        }
        if self.ipv4 {
            w.write_all(b" ipv4")?;
            written += u64_len(b" ipv4");
        }
        if self.ipv6 {
            w.write_all(b" ipv6")?;
            written += u64_len(b" ipv6");
        }
        Ok(written)
    }
}

/// A "getserversExt" message is sent to a master by a client who wants
/// to get a list of servers. It triggers a "getserversExtReponse"
/// message from the master.
#[doc(alias = "getserversExt")]
pub fn write_get_servers_ext<W: Write>(
    mut w: W,
    game_name: &[u8],
    protocol_version: &[u8],
    filter: &GetServersExtFilter,
) -> Result<u64, WriteGetServersExtError> {
    w.write_all(PREFIX)?;
    w.write_all(b"getserversExt")?;
    w.write_all(b" ")?;
    w.write_all(game_name)?;
    w.write_all(b" ")?;
    w.write_all(protocol_version)?;
    let mut written = filter.write(&mut w)?;
    written += PREFIX.len() as u64
        + u64_len(b"getserversExt")
        + u64_len(b" ")
        + u64_len(game_name)
        + u64_len(b" ")
        + u64_len(protocol_version);
    Ok(written)
}

#[derive(Debug)]
pub enum DatagramInfo {
    Single,
    /// (EOT stands for "End Of Transmission")
    Eot,
    Collected,
}

/// A "getserversResponse" message contains a list of IPv4 servers
/// requested by a client.
#[derive(Debug)]
#[doc(alias = "getserversResponse")]
pub struct GetServersResponse {
    pub addresses: Vec<SocketAddrV4>,
    pub kind: DatagramInfo,
}

impl GetServersResponse {
    pub fn parse(bytes: &[u8]) -> Result<(&[u8], GetServersResponse), ParseResponseError> {
        match super::parse::getserversResponse(bytes) {
            Ok((left, parsed)) => Ok((
                left,
                GetServersResponse {
                    addresses: parsed.0,
                    kind: if parsed.1 .0 {
                        DatagramInfo::Eot
                    } else {
                        DatagramInfo::Single
                    },
                },
            )),
            Err(_) => Err(ParseResponseError::InvalidResponse),
        }
    }
}

/// A "getserversExtResponse" message contains a list of IPv4 and/or
/// IPv6 servers requested by a client.
#[derive(Debug)]
#[doc(alias = "getserversExtResponse")]
pub struct GetServersExtResponse {
    pub addresses: Vec<SocketAddr>,
    pub datagram_info: DatagramInfo,
}

impl GetServersExtResponse {
    pub fn parse(bytes: &[u8]) -> Result<(&[u8], GetServersExtResponse), ParseResponseError> {
        match super::parse::getserversExtResponse(bytes) {
            Ok((left, parsed)) => Ok((
                left,
                GetServersExtResponse {
                    addresses: parsed.0,
                    datagram_info: if parsed.1 .0 {
                        DatagramInfo::Eot
                    } else {
                        DatagramInfo::Single
                    },
                },
            )),
            Err(_) => Err(ParseResponseError::InvalidResponse),
        }
    }
}

fn u64_len(a: &[u8]) -> u64 {
    a.len() as u64
}
