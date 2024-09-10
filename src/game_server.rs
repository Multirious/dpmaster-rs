use super::PREFIX;
use std::{collections::HashMap, io::Write};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Write getinfo error")]
pub enum WriteGetInfoError {
    #[error("A challenge must not contains '\\', '/', ';', '\"' or '%'")]
    InvalidChallengeCharacter,
    Io(#[from] std::io::Error),
}

pub fn get_info<W: Write>(mut w: W, challenge: &str) -> Result<(), WriteGetInfoError> {
    if challenge.contains(['\\', '/', ';', '"', '%']) {
        return Err(WriteGetInfoError::InvalidChallengeCharacter);
    }
    w.write_all(PREFIX)?;
    w.write_all(b"getinfo")?;
    w.write_all(b" ")?;
    w.write_all(challenge.as_bytes())?;
    Ok(())
}

#[derive(Debug, Error)]
#[error("Parse game server response error")]
pub enum ParseGameServerResponseError {
    #[error("Invalid response")]
    InvalidResponse,
}

#[derive(Debug)]
pub enum GameServerResponse {
    Info(HashMap<String, String>),
}

pub fn parse_response(
    response: &[u8],
) -> Result<(&[u8], GameServerResponse), ParseGameServerResponseError> {
    if let Ok((left, parsed)) = super::parse::infoReponse(response) {
        let a = parsed
            .into_iter()
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(k).into_owned(),
                    String::from_utf8_lossy(v).into_owned(),
                )
            })
            .collect();
        return Ok((left, GameServerResponse::Info(a)));
    }
    Err(ParseGameServerResponseError::InvalidResponse)
}
