use crate::{parse::PlayerInfo, ParseResponseError};

use super::PREFIX;
use std::borrow::Cow;
use std::io;
use std::{collections::HashMap, io::Write};
use thiserror::Error;

super::define_checked_string! {
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
    pub fn write_all<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(PREFIX)?;
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
    pub key_values: HashMap<String, String>,
}

impl InfoResponse {
    pub fn parse(bytes: &[u8]) -> Result<(&[u8], InfoResponse), ParseResponseError> {
        if let Ok((left, parsed)) = super::parse::infoResponse(bytes) {
            let a = parsed
                .into_iter()
                .map(|(k, v)| {
                    (
                        String::from_utf8_lossy(k).into_owned(),
                        String::from_utf8_lossy(v).into_owned(),
                    )
                })
                .collect();
            return Ok((left, InfoResponse { key_values: a }));
        }
        Err(ParseResponseError::InvalidResponse)
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

    pub fn write_all<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(PREFIX)?;
        writer.write_all(b"getstatus")?;
        writer.write_all(b" ")?;
        writer.write_all(self.challenge.get())?;
        Ok(())
    }
}

#[derive(Debug)]
#[doc(alias = "statusResponse")]
pub struct StatusResponse {
    pub key_values: HashMap<String, String>,
    pub player_infos: Vec<PlayerInfo>,
}

impl StatusResponse {
    pub fn parse(bytes: &[u8]) -> Result<(&[u8], StatusResponse), ParseResponseError> {
        match super::parse::statusResponse(bytes) {
            Ok((left, (kv, player_infos))) => {
                let kv = kv
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            String::from_utf8_lossy(k).into_owned(),
                            String::from_utf8_lossy(v).into_owned(),
                        )
                    })
                    .collect();
                Ok((
                    left,
                    StatusResponse {
                        key_values: kv,
                        player_infos,
                    },
                ))
            }
            Err(e) => {
                println!("{e}");
                Err(ParseResponseError::InvalidResponse)
            }
        }
    }
}
