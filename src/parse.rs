#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while, take_while1},
    character::complete::char,
    combinator::recognize,
    multi::{many0, many1},
    sequence::{pair, tuple},
    Parser,
};
use nom_supreme::{error::ErrorTree, ParserExt};

type IResult<'a, T> = nom::IResult<&'a [u8], T, ErrorTree<&'a [u8]>>;

fn oob(i: &[u8]) -> IResult<&[u8]> {
    tag(super::OOB)
        .context(r#"OOB (Out of band) string b"\xFF\xFF\xFF\xFF""#)
        .parse(i)
}

fn eot(i: &[u8]) -> IResult<&[u8]> {
    tag(b"\\EOT\0\0\0")
        .context(r#"EOT (End of transmission) string b"\\\\EOT\x00\x00\x00""#)
        .parse(i)
}

fn socket_addr_v4(i: &[u8]) -> IResult<SocketAddrV4> {
    let (i, (_, ip, port)) = tuple((
        tag(b"\\").context(r#"IPv4 socket starts with b"\\"#),
        take(4usize).context(r#"4 bytes of IPv4 address"#),
        take(2usize).context(r#"2 bytes of socket address"#)
    )).context(
        r#"Expected IPv4 socket address, starting with b"\\" follows by 4 bytes for address and another 2 bytes for port"#,
    ).parse(i)?;
    let ip = u32::from_be_bytes((&ip[..4]).try_into().unwrap());
    let port = u16::from_be_bytes((&port[..2]).try_into().unwrap());
    let ip = Ipv4Addr::from_bits(ip);
    let socket = SocketAddrV4::new(ip, port);
    Ok((i, socket))
}

fn socket_addr_v6(i: &[u8]) -> IResult<SocketAddrV6> {
    let (i, (_, ip, port)) = tuple((
        tag(b"/").context(r#"IPv6 socket starts with b"/""#),
        take(16usize).context(r#"16 bytes of IPv6 address"#),
        take(2usize).context(r#"2 bytes of socket address"#)
    ))
    .context(r#"Expected IPv6 socket address, starting with b"/" follows by 16 bytes for address and another 2 bytes for port"#)
    .parse(i)?;
    let ip = u128::from_be_bytes((&ip[..16]).try_into().unwrap());
    let port = u16::from_be_bytes((&port[..2]).try_into().unwrap());
    let ip = Ipv6Addr::from_bits(ip);
    let socket = SocketAddrV6::new(ip, port, 0, 0);
    Ok((i, socket))
}

fn socket_addr(i: &[u8]) -> IResult<SocketAddr> {
    alt((
        socket_addr_v4.map(|s| SocketAddr::from(s)),
        socket_addr_v6.map(|s| SocketAddr::from(s)),
    ))
    .context("Alternate between IPv4 and IPv6 address")
    .parse(i)
}

pub struct ContainsEot(pub bool);

enum Either<L, R> {
    Left(L),
    Right(R),
}

pub fn getserversResponse(i: &[u8]) -> IResult<(Vec<SocketAddrV4>, ContainsEot)> {
    let (i, (_, _)) = tuple((
        oob,
        tag(b"getserversResponse").context(r#"b"getserversResponse""#),
    ))
    .context("getserversResponse message")
    .parse(i)?;
    let (i, mut list) = many0(
        alt((
            eot.map(|_| Either::Right(())),
            socket_addr_v4.map(Either::Left),
        ))
        .context("Alternate between EOT and SocketAddrV4"),
    )
    .context("List of SocketAddrV4 with optional EOT")
    .parse(i)?;
    let contains_eot = matches!(list.last(), Some(Either::Right(())));
    if contains_eot {
        list.pop();
    }
    let list = list
        .into_iter()
        .filter_map(|a| match a {
            Either::Left(x) => Some(x),
            Either::Right(()) => None,
        })
        .collect();
    Ok((i, (list, ContainsEot(contains_eot))))
}

pub fn getserversExtResponse(i: &[u8]) -> IResult<(Vec<SocketAddr>, ContainsEot)> {
    let (i, (_, _)) = tuple((
        oob,
        tag(b"getserversExtResponse").context(r#"b"getserversExtResponse""#),
    ))
    .context("getserversExtResponse message")
    .parse(i)?;
    let (i, mut list) = many0(
        alt((
            eot.map(|_| Either::Right(())),
            socket_addr.map(Either::Left),
        ))
        .context("Alternate between EOT and SocketAddr"),
    )
    .context("List of SocketAddr with optional EOT")
    .parse(i)?;
    let contains_eot = matches!(list.last(), Some(Either::Right(())));
    if contains_eot {
        list.pop();
    }
    let list = list
        .into_iter()
        .filter_map(|a| match a {
            Either::Left(x) => Some(x),
            Either::Right(()) => None,
        })
        .collect();
    Ok((i, (list, ContainsEot(contains_eot))))
}

pub fn key_value_map(i: &[u8]) -> IResult<HashMap<&[u8], &[u8]>> {
    let (i, o) = many1(tuple((
        tag(b"\\").context(r#"b"\\" key prefix"#),
        take_while(|b: u8| b != b'\\').context(r#"Take while bytes is not b'\\'"#),
        tag(b"\\").context(r#"b"\\" value prefix"#),
        take_while(|b: u8| b != b'\\' && b != b'\n')
            .context(r#"Take while byte is not b'\\' or b'\n'"#),
    )))
    .context("Key value map")
    .parse(i)?;
    let map = o.into_iter().map(|(_, k, _, v)| (k, v)).collect();
    Ok((i, map))
}

pub fn infoResponse(i: &[u8]) -> IResult<HashMap<&[u8], &[u8]>> {
    let (i, (_, _, map)) = tuple((
        oob,
        tag(b"infoResponse\n").context(r#"b"infoResponse\n""#),
        key_value_map,
    ))
    .context("infoResponse")
    .parse(i)?;
    Ok((i, map))
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub frags: i32,
    pub ping: i32,
    pub name: String,
    pub team: i32,
}

fn dquoted_string(i: &[u8]) -> IResult<&[u8]> {
    let (i, (_, text, _)) = tuple((
        tag(b"\"").context("Double quote"),
        take_until(b"\"".as_slice()).context("Take until another double quote"),
        tag(b"\"").context("End double quote"),
    ))
    .context("Bytes in double quotes (\"...\")")
    .parse(i)?;
    Ok((i, text))
}

fn dquoted_ascii(i: &[u8]) -> IResult<&str> {
    dquoted_string
        .map_res(|bytes| std::str::from_utf8(bytes))
        .parse(i)
}

fn int(i: &[u8]) -> IResult<&[u8]> {
    fn take_ascii_digits(i: &[u8]) -> IResult<&[u8]> {
        take_while1(|b: u8| b.is_ascii_digit())
            .context("Ascii digits [0-9]")
            .parse(i)
    }
    alt((
        recognize(pair(
            char('-').context("Minus sign"),
            take_ascii_digits.cut(),
        ))
        .context("Negative signed integer"),
        take_ascii_digits.context("Posivtive signed integer"),
    ))
    .context("Integer")
    .parse(i)
}

fn player_infos(i: &[u8]) -> IResult<Vec<PlayerInfo>> {
    fn player(i: &[u8]) -> IResult<PlayerInfo> {
        let info = tuple((
            int.context("Frags"),
            tag(b" ").context("Space after frags"),
            int.context("Ping"),
            tag(b" ").context("Space after ping"),
            dquoted_ascii.context("Name"),
            tag(b" ").context("Space after name"),
            int.context("Team"),
            tag(b"\n").context("New line after team"),
        ))
        .context("Player info")
        .parse(i);
        let (i, (frags, _, ping, _, name, _, team, _)) = info?;
        Ok((
            i,
            PlayerInfo {
                frags: std::str::from_utf8(frags).unwrap().parse().unwrap(),
                ping: std::str::from_utf8(ping).unwrap().parse().unwrap(),
                name: name.to_string(),
                team: std::str::from_utf8(team).unwrap().parse().unwrap(),
            },
        ))
    }

    many1(player).context("List of player info").parse(i)
}

#[allow(clippy::type_complexity)]
pub fn statusResponse(i: &[u8]) -> IResult<(HashMap<&[u8], &[u8]>, Vec<PlayerInfo>)> {
    alt((
        tuple((
            oob,
            tag(b"statusResponse\n").context(r#"b"statusResponse\n""#),
            key_value_map.cut(),
            tag(b"\n").context("Newline seperated player info"),
            player_infos.cut(),
        ))
        .map(|(_oob, _msg, kv, _nl, player_infos)| (kv, player_infos))
        .context("statusResponse with player infos"),
        tuple((
            oob,
            tag(b"statusResponse\n").context(r#"b"statusResponse\n""#),
            key_value_map.cut(),
        ))
        .map(|(_oob, _msg, kv)| (kv, vec![]))
        .context("statusResponse without player infos"),
    ))
    .context("statusResponse with possibly player infos")
    .parse(i)
}
