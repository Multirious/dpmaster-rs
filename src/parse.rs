#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_until, take_while},
    multi::{many0, many1},
    sequence::tuple,
    Parser,
};

type IResult<'a, T> = nom::IResult<&'a [u8], T>;

fn command_prefix(i: &[u8]) -> IResult<&[u8]> {
    tag(super::PREFIX)(i)
}

fn eot(i: &[u8]) -> IResult<&[u8]> {
    tag(b"\\EOT\0\0\0")(i)
}

fn socket_addr_v4(i: &[u8]) -> IResult<SocketAddrV4> {
    let (i, (_, ip, port)) = tuple((tag(b"\\"), take(4usize), take(2usize)))(i)?;
    let ip = Ipv4Addr::new(
        u8::from_be(ip[0]),
        u8::from_be(ip[1]),
        u8::from_be(ip[2]),
        u8::from_be(ip[3]),
    );
    let socket = SocketAddrV4::new(ip, u16::from_be_bytes((&port[..2]).try_into().unwrap()));
    Ok((i, socket))
}

fn socket_addr_v6(i: &[u8]) -> IResult<SocketAddrV6> {
    let (i, (_, ip, port)) = tuple((tag(b"/"), take(16usize), take(2usize)))(i)?;
    let ip = Ipv6Addr::new(
        u16::from_be_bytes((&ip[0..2]).try_into().unwrap()),
        u16::from_be_bytes((&ip[2..4]).try_into().unwrap()),
        u16::from_be_bytes((&ip[4..6]).try_into().unwrap()),
        u16::from_be_bytes((&ip[6..8]).try_into().unwrap()),
        u16::from_be_bytes((&ip[8..10]).try_into().unwrap()),
        u16::from_be_bytes((&ip[10..12]).try_into().unwrap()),
        u16::from_be_bytes((&ip[12..14]).try_into().unwrap()),
        u16::from_be_bytes((&ip[14..16]).try_into().unwrap()),
    );
    let socket = SocketAddrV6::new(
        ip,
        u16::from_be_bytes((&port[..2]).try_into().unwrap()),
        0, // hm
        0, // hmmmmmm
    );
    Ok((i, socket))
}

fn socket_addr(i: &[u8]) -> IResult<SocketAddr> {
    if let Ok((i, s)) = socket_addr_v6(i) {
        return Ok((i, SocketAddr::V6(s)));
    }
    match socket_addr_v4(i) {
        Ok((i, s)) => Ok((i, SocketAddr::V4(s))),
        Err(e) => Err(e),
    }
}

pub struct ContainsEot(pub bool);

enum Either<L, R> {
    Left(L),
    Right(R),
}

pub fn getserversResponse(i: &[u8]) -> IResult<(Vec<SocketAddrV4>, ContainsEot)> {
    let (i, (_, _)) = tuple((command_prefix, tag(b"getserversResponse")))(i)?;
    let (i, mut list) = many0(alt((
        eot.map(|_| Either::Right(())),
        socket_addr_v4.map(Either::Left),
    )))(i)?;
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
    let (i, (_, _)) = tuple((command_prefix, tag(b"getserversExtResponse")))(i)?;
    let (i, mut list) = many0(alt((
        eot.map(|_| Either::Right(())),
        socket_addr.map(Either::Left),
    )))(i)?;
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
        tag(b"\\"),
        take_while(|b: u8| b != b'\\'),
        tag(b"\\"),
        take_while(|b: u8| b != b'\\' && b != b'\n'),
    )))(i)?;
    let map = o.into_iter().map(|(_, k, _, v)| (k, v)).collect();
    Ok((i, map))
}

pub fn infoResponse(i: &[u8]) -> IResult<HashMap<&[u8], &[u8]>> {
    let (i, (_, _, map)) = tuple((command_prefix, tag(b"infoResponse\n"), key_value_map))(i)?;
    Ok((i, map))
}

#[derive(Debug)]
pub struct PlayerInfo {
    pub frags: i32,
    pub ping: i32,
    pub name: String,
    pub team: i32,
}

fn ascii_in_dquote(i: &[u8]) -> IResult<&[u8]> {
    let var_name = tuple((tag(b"\""), take_until(b"\"".as_slice())))(i);
    let (i, (_, o)) = var_name?;
    let i = &i[1..];
    Ok((i, o))
}

fn num_with_sign(i: &[u8]) -> IResult<&[u8]> {
    fn num_neg(i: &[u8]) -> IResult<&[u8]> {
        let (i2, (_, o)) = tuple((tag(b"-"), take_while(|b: u8| b.is_ascii_digit())))(i)?;
        Ok((i2, &i[..(o.len() + 1)]))
    }

    alt((num_neg, take_while(|b: u8| b.is_ascii_digit())))(i)
}

fn player_infos(i: &[u8]) -> IResult<Vec<PlayerInfo>> {
    fn player(i: &[u8]) -> IResult<PlayerInfo> {
        let test = tuple((
            num_with_sign,
            tag(b" "),
            num_with_sign,
            tag(b" "),
            ascii_in_dquote,
            tag(b" "),
            num_with_sign,
            tag(b"\n"),
        ))(i);
        let (i, (frags, _, ping, _, name, _, team, _)) = test?;
        Ok((
            i,
            PlayerInfo {
                frags: std::str::from_utf8(frags).unwrap().parse().unwrap(),
                ping: std::str::from_utf8(ping).unwrap().parse().unwrap(),
                name: String::from_utf8(name.to_vec()).unwrap(),
                team: std::str::from_utf8(team).unwrap().parse().unwrap(),
            },
        ))
    }

    many0(player)(i)
}

pub fn statusResponse(i: &[u8]) -> IResult<(HashMap<&[u8], &[u8]>, Vec<PlayerInfo>)> {
    let (i, (_, _, kv)) = tuple((command_prefix, tag(b"statusResponse\n"), key_value_map))(i)?;
    let (i, players) = if let Ok((i, (_, players))) = tuple((tag(b"\n"), player_infos))(i) {
        (i, players)
    } else {
        (i, vec![])
    };
    Ok((i, (kv, players)))
}
