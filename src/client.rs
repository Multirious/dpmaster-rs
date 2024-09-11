use std::{
    io::{self, Cursor},
    net::SocketAddr,
    time::Duration,
};
use thiserror::Error;
use tokio::{net::UdpSocket, select};

use crate::{
    game_server_commands::{self as gs_cmd, InfoResponse, StatusResponse},
    master_server_commands as ms_cmd,
};

const MAX_PACKET_LEN: usize = 1400;

#[derive(Debug, Error)]
#[error("Client error")]
pub enum ClientError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("Message timeout")]
    Timeout,
    #[error("{0}")]
    WriteGetServersError(#[from] ms_cmd::WriteGetServersError),
    #[error("{0}")]
    WriteGetServersExtError(#[from] ms_cmd::WriteGetServersExtError),
    #[error("{0}")]
    WriteGetInfoError(#[from] gs_cmd::WriteGetInfoError),
    #[error("{0}")]
    WriteGetStatusError(#[from] gs_cmd::WriteGetStatusError),
    #[error("{0}")]
    ParseResponseError(#[from] crate::ParseResponseError),
}

#[derive(Debug)]
pub struct Master {
    pub socket: UdpSocket,
    pub addr: SocketAddr,
    send_buf: Cursor<Vec<u8>>,
    recv_buf: Cursor<Vec<u8>>,
}

impl Master {
    pub fn with_socket(socket: UdpSocket, master: SocketAddr) -> Master {
        Master {
            socket,
            addr: master,
            send_buf: Cursor::new(vec![0; MAX_PACKET_LEN]),
            recv_buf: Cursor::new(vec![0; MAX_PACKET_LEN]),
        }
    }

    pub async fn new(master: SocketAddr) -> io::Result<Master> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self::with_socket(socket, master))
    }

    #[doc(alias = "getservers")]
    pub async fn get_servers(
        &mut self,
        game_name: Option<&[u8]>,
        protocol_version: &[u8],
        filter: &ms_cmd::GetServersFilter<'_>,
        timeout: Duration,
    ) -> Result<ms_cmd::GetServersResponse, ClientError> {
        self.send_buf.set_position(0);
        let written =
            ms_cmd::write_get_servers(&mut self.send_buf, game_name, protocol_version, filter)?
                as usize;
        self.socket
            .send_to(&self.send_buf.get_ref()[0..written], &self.addr)
            .await?;

        self.recv_buf.set_position(0);

        let mut writes: Vec<usize> = vec![];
        loop {
            select! {
                Ok(written) = self.socket.recv(self.recv_buf.get_mut()) => {
                    writes.push(written);
                    let pos = self.recv_buf.position() as usize;
                    let end_pos = pos + written;
                    let last_7_bytes = &self.recv_buf.get_ref()[(end_pos - 7)..end_pos];
                    let has_eot = last_7_bytes == b"\\EOT\0\0\0";
                    if has_eot {
                        break;
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    break;
                }
            }
        }
        let result = writes
            .into_iter()
            .scan(0, |pos, written| {
                let bytes = &self.recv_buf.get_ref()[(*pos)..(*pos + written)];
                *pos += written;
                Some(bytes)
            })
            .map(ms_cmd::GetServersResponse::parse)
            .filter_map(Result::ok)
            .map(|(_, x)| x)
            .fold(
                ms_cmd::GetServersResponse {
                    addresses: vec![],
                    kind: ms_cmd::DatagramInfo::Collected,
                },
                |mut acc, res| {
                    acc.addresses.extend_from_slice(&res.addresses[..]);
                    acc
                },
            );
        Ok(result)
    }

    #[doc(alias = "getserversExt")]
    pub async fn get_servers_ext(
        &mut self,
        game_name: &[u8],
        protocol_version: &[u8],
        filter: &ms_cmd::GetServersExtFilter<'_>,
        timeout: Duration,
    ) -> Result<ms_cmd::GetServersExtResponse, ClientError> {
        self.send_buf.set_position(0);
        let written =
            ms_cmd::write_get_servers_ext(&mut self.send_buf, game_name, protocol_version, filter)?
                as usize;
        self.socket
            .send_to(&self.send_buf.get_ref()[0..written], &self.addr)
            .await?;

        self.recv_buf.set_position(0);

        let mut writes: Vec<usize> = vec![];
        loop {
            select! {
                Ok(written) = self.socket.recv(self.recv_buf.get_mut()) => {
                    writes.push(written);
                    let pos = self.recv_buf.position() as usize;
                    let end_pos = pos + written;
                    let last_7_bytes = &self.recv_buf.get_ref()[(end_pos - 7)..end_pos];
                    let has_eot = last_7_bytes == b"\\EOT\0\0\0";
                    if has_eot {
                        break;
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    break;
                }
            }
        }
        let result = writes
            .into_iter()
            .scan(0, |pos, written| {
                let bytes = &self.recv_buf.get_ref()[(*pos)..(*pos + written)];
                *pos += written;
                Some(bytes)
            })
            .map(ms_cmd::GetServersExtResponse::parse)
            .filter_map(Result::ok)
            .map(|(_, x)| x)
            .fold(
                ms_cmd::GetServersExtResponse {
                    addresses: vec![],
                    datagram_info: ms_cmd::DatagramInfo::Collected,
                },
                |mut acc, res| {
                    acc.addresses.extend_from_slice(&res.addresses[..]);
                    acc
                },
            );
        Ok(result)
    }

    pub async fn get_info(&self) {}
}

#[derive(Debug)]
pub struct Game {
    pub socket: UdpSocket,
    pub addr: SocketAddr,
    send_buf: Cursor<Vec<u8>>,
    recv_buf: Cursor<Vec<u8>>,
}

impl Game {
    pub fn with_socket(socket: UdpSocket, game: SocketAddr) -> Game {
        Game {
            socket,
            addr: game,
            send_buf: Cursor::new(vec![0; MAX_PACKET_LEN]),
            recv_buf: Cursor::new(vec![0; MAX_PACKET_LEN]),
        }
    }

    pub async fn new(game: SocketAddr) -> io::Result<Game> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self::with_socket(socket, game))
    }

    #[doc(alias = "getinfo")]
    pub async fn get_info(
        &mut self,
        challenge: &[u8],
        timeout: Duration,
    ) -> Result<InfoResponse, ClientError> {
        self.send_buf.set_position(0);
        let written = gs_cmd::write_get_info(&mut self.send_buf, challenge)? as usize;

        self.socket
            .send_to(&self.send_buf.get_ref()[0..written], self.addr)
            .await?;

        let written = loop {
            select! {
                Ok((written, socket)) = {
                    self.recv_buf.set_position(0);
                    self.socket.recv_from(self.recv_buf.get_mut())
                } => {
                    self.recv_buf.set_position(written as u64);
                    if socket != self.addr {
                        continue;
                    }
                    break written;
                }
                _ = tokio::time::sleep(timeout) => {
                    return Err(ClientError::Timeout);
                }
            }
        };
        let (_, reponse) = InfoResponse::parse(&self.recv_buf.get_ref()[0..written])?;
        Ok(reponse)
    }

    #[doc(alias = "getstatus")]
    pub async fn get_status(
        &mut self,
        challenge: &[u8],
        timeout: Duration,
    ) -> Result<StatusResponse, ClientError> {
        self.send_buf.set_position(0);
        let written = gs_cmd::write_get_status(&mut self.send_buf, challenge)? as usize;

        self.socket
            .send_to(&self.send_buf.get_ref()[0..written], self.addr)
            .await?;

        let written = loop {
            select! {
                Ok((written, socket)) = {
                    self.recv_buf.set_position(0);
                    self.socket.recv_from(self.recv_buf.get_mut())
                } => {
                    self.recv_buf.set_position(written as u64);
                    if socket != self.addr {
                        continue;
                    }
                    break written;
                }
                _ = tokio::time::sleep(timeout) => {
                    return Err(ClientError::Timeout);
                }
            }
        };
        let (_, reponse) = StatusResponse::parse(&self.recv_buf.get_ref()[0..written])?;
        Ok(reponse)
    }
}
