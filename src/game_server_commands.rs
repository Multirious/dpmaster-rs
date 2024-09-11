use crate::{parse::PlayerInfo, ParseResponseError};

use super::PREFIX;
use std::{collections::HashMap, io::Write};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Write getinfo error")]
pub enum WriteGetInfoError {
    #[error(
        "A challenge must only contains ASCII characters but exclude '\\', '/', ';', '\"' and '%'"
    )]
    InvalidChallengeCharacter,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// This message is sent by a master to a server, usually in response
/// to an "hearbeat" by this very server. It is used by the master to
/// trigger the sending of an "infoResponse" from the server. The
/// challenge string is necessary to authenticate the server's
/// corresponding "infoResponse".
#[doc(alias = "getinfo")]
pub fn write_get_info<W: Write>(mut w: W, challenge: &[u8]) -> Result<u64, WriteGetInfoError> {
    if !challenge.iter().all(|b| match *b {
        b'\\' | b'/' | b';' | b'"' | b'%' => false,
        33..=126 => true,
        _ => false,
    }) {
        return Err(WriteGetInfoError::InvalidChallengeCharacter);
    }
    w.write_all(PREFIX)?;
    w.write_all(b"getinfo")?;
    w.write_all(b" ")?;
    w.write_all(challenge)?;
    let written = u64_len(PREFIX) + u64_len(b"getinfo") + u64_len(b" ") + u64_len(challenge);
    Ok(written)
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

#[derive(Debug, Error)]
#[error("Write getstatus error")]
pub enum WriteGetStatusError {
    #[error(
        "A challenge must only contains ASCII characters but exclude '\\', '/', ';', '\"' and '%'"
    )]
    InvalidChallengeCharacter,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[doc(alias = "getstatus")]
pub fn write_get_status<W: Write>(mut w: W, challenge: &[u8]) -> Result<u64, WriteGetStatusError> {
    if !challenge.iter().all(|b| match *b {
        b'\\' | b'/' | b';' | b'"' | b'%' => false,
        33..=126 => true,
        _ => false,
    }) {
        return Err(WriteGetStatusError::InvalidChallengeCharacter);
    }
    w.write_all(PREFIX)?;
    w.write_all(b"getstatus")?;
    w.write_all(b" ")?;
    w.write_all(challenge)?;
    let written = u64_len(PREFIX) + u64_len(b"getstatus") + u64_len(b" ") + u64_len(challenge);
    Ok(written)
}

#[derive(Debug)]
#[doc(alias = "infoResponse")]
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

fn u64_len(a: &[u8]) -> u64 {
    a.len() as u64
}
