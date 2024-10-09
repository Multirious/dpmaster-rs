//! Original protocol technical information [here](https://github.com/kphillisjr/dpmaster/blob/master/doc/techinfo.txt).

use nom::Parser;
use nom_supreme::final_parser::final_parser;
use std::{
    borrow::Cow,
    collections::HashMap,
    io::{self, Write},
    net::{SocketAddr, SocketAddrV4},
};
use thiserror::Error;

mod parse;

const OOB: &[u8] = b"\xFF\xFF\xFF\xFF";

macro_rules! define_checked_string {
    (
        $error_message:literal,
        $error_name:ident,
        $struct_name:ident,
        $arg_name:ident,
        $check:expr
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
        #[error($error_message)]
        pub struct $error_name;

        #[derive(Debug)]
        pub struct $struct_name<'a>(Cow<'a, [u8]>);
        impl $struct_name<'_> {
            pub fn new($arg_name: Cow<'_, [u8]>) -> Result<$struct_name<'_>, $error_name> {
                if !$arg_name.iter().all($check) {
                    return Err($error_name);
                }
                Ok($struct_name($arg_name))
            }

            #[inline]
            pub fn new_unchecked($arg_name: Cow<'_, [u8]>) -> $struct_name<'_> {
                $struct_name($arg_name)
            }

            #[inline]
            pub fn get(&self) -> &Cow<'_, [u8]> {
                &self.0
            }
        }
        impl AsRef<[u8]> for $struct_name<'_> {
            #[inline]
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }
        impl<'a> TryFrom<&'a [u8]> for $struct_name<'a> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: &'a [u8]) -> Result<$struct_name<'a>, Self::Error> {
                $struct_name::new(Cow::Borrowed(value))
            }
        }
        impl TryFrom<Vec<u8>> for $struct_name<'_> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                $struct_name::new(Cow::Owned(value))
            }
        }
        impl<'a> TryFrom<&'a str> for $struct_name<'a> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: &'a str) -> Result<$struct_name<'a>, Self::Error> {
                $struct_name::new(Cow::Borrowed(value.as_bytes()))
            }
        }
        impl TryFrom<String> for $struct_name<'_> {
            type Error = $error_name;

            #[inline]
            fn try_from(value: String) -> Result<Self, Self::Error> {
                $struct_name::new(Cow::Owned(value.into_bytes()))
            }
        }
    };
}

define_checked_string! {
    "A challenge must only contains ASCII characters but exclude '\\', '/', ';', '\"' and '%'",
    NewChallengeError,
    Challenge,
    challenge,
    |b| match *b {
        b'\\' | b'/' | b';' | b'"' | b'%' => false,
        33..=126 => true,
        _ => false,
    }
}
define_checked_string! {
    "Protocol strings cannot contain a new-line (\\n)",
    NewProtocolStringError,
    ProtocolString,
    protocol_string,
    |b| *b != b'\n'
}
define_checked_string! {
    "Game names must not contain any whitespace",
    NewGameNameError,
    GameName,
    game_name,
    |b| !b.is_ascii_whitespace()
}
define_checked_string! {
    "Protocol version must only be a number",
    NewProtocolVersionError,
    ProtocolVersion,
    protocol_version,
    |b| b.is_ascii_digit()
}

type ParseResult<'a, T> = std::result::Result<T, nom_supreme::error::ErrorTree<&'a [u8]>>;

/// This message is sent by a master to a server, usually in response
/// to an "hearbeat" by this very server. It is used by the master to
/// trigger the sending of an "infoResponse" from the server. The
/// challenge string is necessary to authenticate the server's
/// corresponding "infoResponse".
#[doc(alias = "getinfo")]
pub struct GetInfo<'a> {
    pub challenge: Challenge<'a>,
}

impl GetInfo<'_> {
    pub fn new(challenge: Challenge) -> GetInfo<'_> {
        GetInfo { challenge }
    }
    pub fn write_all_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(OOB)?;
        writer.write_all(b"getinfo")?;
        writer.write_all(b" ")?;
        writer.write_all(self.challenge.as_ref())?;
        Ok(())
    }
}

/// An "infoResponse" message is the reponse to a "getinfo" request.
/// It contains an infostring including the most important information
/// about the current server state.
#[derive(Debug)]
#[doc(alias = "infoResponse")]
pub struct InfoResponse {
    pub key_values: HashMap<Vec<u8>, Vec<u8>>,
}

impl InfoResponse {
    pub fn parse(bytes: &[u8]) -> ParseResult<InfoResponse> {
        final_parser(parse::infoResponse.map(|key_values| {
            InfoResponse {
                key_values: key_values
                    .into_iter()
                    .map(|(k, v)| (k.to_vec(), v.to_vec()))
                    .collect(),
            }
        }))(bytes)
    }
}

#[doc(alias = "getstatus")]
pub struct GetStatus<'a> {
    pub challenge: Challenge<'a>,
}
impl GetStatus<'_> {
    pub fn new(challenge: Challenge) -> GetStatus<'_> {
        GetStatus { challenge }
    }

    pub fn write_all_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(OOB)?;
        writer.write_all(b"getstatus")?;
        writer.write_all(b" ")?;
        writer.write_all(self.challenge.get())?;
        Ok(())
    }
}

#[derive(Debug)]
#[doc(alias = "statusResponse")]
pub struct StatusResponse {
    pub key_values: HashMap<Vec<u8>, Vec<u8>>,
    pub player_infos: Vec<PlayerInfo>,
}
pub use parse::PlayerInfo;

impl StatusResponse {
    pub fn parse(bytes: &[u8]) -> ParseResult<StatusResponse> {
        final_parser(parse::statusResponse.map(|(kv, player_infos)| {
            StatusResponse {
                key_values: kv
                    .into_iter()
                    .map(|(k, v)| (k.to_vec(), v.to_vec()))
                    .collect(),
                player_infos,
            }
        }))(bytes)
    }
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
    pub fn write_all_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(OOB)?;
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
    fn write_all_to<W: Write>(&self, mut w: W) -> io::Result<()> {
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
    pub fn write_all_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(OOB)?;
        writer.write_all(b"getservers")?;
        if let Some(ref game_name) = self.game_name {
            writer.write_all(b" ")?;
            writer.write_all(game_name.as_ref())?;
        }
        writer.write_all(b" ")?;
        writer.write_all(self.protocol_version.as_ref())?;
        self.filter.write_all_to(&mut writer)?;
        Ok(())
    }
}

/// A "getserversResponse" message contains a list of IPv4 servers
/// requested by a client.
#[derive(Debug)]
#[doc(alias = "getserversResponse")]
pub struct GetServersResponse {
    pub addresses: Vec<SocketAddrV4>,
    pub eot: bool,
}

impl GetServersResponse {
    pub fn parse(bytes: &[u8]) -> ParseResult<GetServersResponse> {
        final_parser(parse::getserversResponse.map(|o| GetServersResponse {
            addresses: o.0,
            eot: o.1 .0,
        }))(bytes)
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
    fn write_all_to<W: Write>(&self, mut w: W) -> io::Result<()> {
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
    pub fn write_all_to<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(OOB)?;
        writer.write_all(b"getserversExt")?;
        writer.write_all(b" ")?;
        writer.write_all(self.game_name.as_ref())?;
        writer.write_all(b" ")?;
        writer.write_all(self.protocol_version.as_ref())?;
        self.filter.write_all_to(&mut writer)?;
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
    pub fn parse(bytes: &[u8]) -> ParseResult<GetServersExtResponse> {
        final_parser(parse::getserversExtResponse.map(|o| GetServersExtResponse {
            addresses: o.0,
            datagram_info: if o.1 .0 {
                DatagramInfo::Eot
            } else {
                DatagramInfo::Single
            },
        }))(bytes)
    }
}
