//! Original protocol technical information [here](https://github.com/kphillisjr/dpmaster/blob/master/doc/techinfo.txt).

mod parse;

const PREFIX: &[u8] = b"\xFF\xFF\xFF\xFF";

/// All appropriate commands that is possible to send to a game server.
pub mod game_server_commands;
/// All appropriate commands that is possible to send to a master server.
pub mod master_server_commands;

#[derive(Debug, thiserror::Error)]
#[error("Parse response error")]
pub enum ParseResponseError {
    #[error("The input bytes is invalid for this response type")]
    InvalidResponse,
}

#[cfg(feature = "tokio")]
pub mod client;
