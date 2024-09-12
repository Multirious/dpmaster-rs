use std::{
    borrow::Cow,
    io::{self, Write},
    net::{SocketAddr, SocketAddrV4},
};
use thiserror::Error;

use crate::ParseResponseError;

use super::PREFIX;

super::define_checked_string! {
    "Protocol strings cannot contain a new-line (\\n)",
    NewProtocolStringError,
    ProtocolString,
    protocol_string,
    |b| *b != b'\n'
}
super::define_checked_string! {
    "Game names must not contain any whitespace",
    NewGameNameError,
    GameName,
    game_name,
    |b| !b.is_ascii_whitespace()
}
super::define_checked_string! {
    "Protocol version must only be a number",
    NewProtocolVersionError,
    ProtocolVersion,
    protocol_version,
    |b| b.is_ascii_digit()
}

#[derive(Debug)]
pub enum DatagramInfo {
    Single,
    /// (EOT stands for "End Of Transmission")
    Eot,
    Collected,
}

/// The heartbeat is sent by a server when it wants to get noticed by a
/// master. A server should send an heartbeat each time it becomes empty
/// or full, or stop being empty or full, plus it should make sure the
/// master gets at least one heartbeat from it every 10 or 15 minutes,
/// so the master doesn't remove it from its list of active servers.
#[doc(alias = "heartbeat")]
#[derive(Debug)]
pub struct HeartBeat<'a> {
    pub protocol_string: ProtocolString<'a>,
}

impl HeartBeat<'_> {
    pub fn write_all<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(PREFIX)?;
        writer.write_all(b"heartbeat")?;
        writer.write_all(b" ")?;
        writer.write_all(self.protocol_string.get())?;
        writer.write_all(b"\n")?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct GetServersFilter<'a> {
    pub empty: bool,
    pub full: bool,
    pub gametype: Option<Cow<'a, [u8]>>,
}
impl GetServersFilter<'_> {
    fn write_all<W: Write>(&self, mut w: W) -> io::Result<()> {
        if self.empty {
            w.write_all(b" empty")?;
        }
        if self.full {
            w.write_all(b" full")?;
        }
        if let Some(ref game_type) = self.gametype {
            w.write_all(b" ")?;
            w.write_all(game_type.as_ref())?;
        }
        Ok(())
    }
}
/// A "getservers" message is sent to a master by a client who wants
/// to get a list of servers. It triggers a "getserversReponse"
/// message from the master.
#[doc(alias = "getservers")]
#[derive(Debug)]
pub struct GetServers<'a> {
    pub game_name: Option<GameName<'a>>,
    pub protocol_version: ProtocolVersion<'a>,
    pub filter: GetServersFilter<'a>,
}

impl GetServers<'_> {
    pub fn write_all<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(PREFIX)?;
        writer.write_all(b"getservers")?;
        if let Some(ref game_name) = self.game_name {
            writer.write_all(b" ")?;
            writer.write_all(game_name.as_ref())?;
        }
        writer.write_all(b" ")?;
        writer.write_all(self.protocol_version.as_ref())?;
        self.filter.write_all(&mut writer)?;
        Ok(())
    }
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

#[derive(Debug, Default)]
pub struct GetServersExtFilter<'a> {
    pub empty: bool,
    pub full: bool,
    pub gametype: Option<Cow<'a, [u8]>>,
    pub ipv4: bool,
    pub ipv6: bool,
}
impl GetServersExtFilter<'_> {
    fn write_all<W: Write>(&self, mut w: W) -> io::Result<()> {
        if self.empty {
            w.write_all(b" empty")?;
        }
        if self.full {
            w.write_all(b" full")?;
        }
        if let Some(game_type) = &self.gametype {
            w.write_all(b" ")?;
            w.write_all(game_type.as_ref())?;
        }
        if self.ipv4 {
            w.write_all(b" ipv4")?;
        }
        if self.ipv6 {
            w.write_all(b" ipv6")?;
        }
        Ok(())
    }
}

/// A "getserversExt" message is sent to a master by a client who wants
/// to get a list of servers. It triggers a "getserversExtReponse"
/// message from the master.
#[doc(alias = "getserversExt")]
#[derive(Debug)]
pub struct GetServersExt<'a> {
    pub game_name: GameName<'a>,
    pub protocol_version: ProtocolVersion<'a>,
    pub filter: GetServersExtFilter<'a>,
}
impl GetServersExt<'_> {
    pub fn write_all<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(PREFIX)?;
        writer.write_all(b"getserversExt")?;
        writer.write_all(b" ")?;
        writer.write_all(self.game_name.as_ref())?;
        writer.write_all(b" ")?;
        writer.write_all(self.protocol_version.as_ref())?;
        self.filter.write_all(&mut writer)?;
        Ok(())
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
