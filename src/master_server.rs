use std::{
    io::Write,
    net::{SocketAddr, SocketAddrV4},
};
use thiserror::Error;

use super::PREFIX;

#[derive(Debug, Error)]
#[error("Write getservers error")]
pub enum WriteGetServersError {
    Io(#[from] std::io::Error),
}

pub fn get_servers<W: Write>(
    mut w: W,
    name: &str,
    protocol_version: &str,
    full: bool,
    empty: bool,
) -> Result<(), WriteGetServersError> {
    w.write_all(PREFIX)?;
    w.write_all(b"getservers")?;
    w.write_all(b" ")?;
    w.write_all(name.as_bytes())?;
    w.write_all(b" ")?;
    w.write_all(protocol_version.as_bytes())?;
    if full {
        w.write_all(b" ")?;
        w.write_all(b"full")?;
    }
    if empty {
        w.write_all(b" ")?;
        w.write_all(b"empty")?;
    }
    Ok(())
}

#[derive(Debug, Error)]
#[error("Write getserversExt error")]
pub enum WriteGetServersExtError {
    Io(#[from] std::io::Error),
}

#[allow(non_snake_case)]
pub fn get_servers_ext<W: Write>(
    mut w: W,
    name: &str,
    protocol_version: &str,
    full: bool,
    empty: bool,
) -> Result<(), WriteGetServersExtError> {
    w.write_all(PREFIX)?;
    w.write_all(b"getserversExt")?;
    w.write_all(b" ")?;
    w.write_all(name.as_bytes())?;
    w.write_all(b" ")?;
    w.write_all(protocol_version.as_bytes())?;
    if full {
        w.write_all(b" ")?;
        w.write_all(b"full")?;
    }
    if empty {
        w.write_all(b" ")?;
        w.write_all(b"empty")?;
    }
    Ok(())
}

#[derive(Debug, Error)]
#[error("Parse master server response error")]
pub enum ParseMasterServerResponseError {
    #[error("Invalid response")]
    InvalidResponse,
}

#[derive(Debug)]
pub enum MasterServerResponse {
    Servers(Vec<SocketAddrV4>),
    ServersExt(Vec<SocketAddr>),
}

pub fn parse_response(
    response: &[u8],
) -> Result<(&[u8], MasterServerResponse), ParseMasterServerResponseError> {
    if let Ok((left, parsed)) = super::parse::getserversResponse(response) {
        return Ok((left, MasterServerResponse::Servers(parsed)));
    }
    if let Ok((left, parsed)) = super::parse::getserversExtResponse(response) {
        return Ok((left, MasterServerResponse::ServersExt(parsed)));
    }
    Err(ParseMasterServerResponseError::InvalidResponse)
}
