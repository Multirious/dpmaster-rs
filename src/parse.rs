#![allow(non_snake_case)]

use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use nom::{
    bytes::complete::{tag, take, take_while},
    multi::{many1, many_till},
    sequence::tuple,
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

pub fn getserversResponse(i: &[u8]) -> IResult<Vec<SocketAddrV4>> {
    let (i, (_, _, (sockets, _))) = tuple((
        command_prefix,
        tag(b"getserversResponse"),
        many_till(socket_addr_v4, eot),
    ))(i)?;
    Ok((i, sockets))
}

pub fn getserversExtResponse(i: &[u8]) -> IResult<Vec<SocketAddr>> {
    let (i, (_, _, (sockets, _))) = tuple((
        command_prefix,
        tag(b"getserversExtResponse"),
        many_till(socket_addr, eot),
    ))(i)?;
    Ok((i, sockets))
}

pub fn key_value(i: &[u8]) -> IResult<HashMap<&[u8], &[u8]>> {
    let (i, o) = many1(tuple((
        tag(b"\\"),
        take_while(|b: u8| b != b'\\'),
        tag(b"\\"),
        take_while(|b: u8| b != b'\\'),
    )))(i)?;
    let map = o.into_iter().map(|(_, k, _, v)| (k, v)).collect();
    Ok((i, map))
}

pub fn infoReponse(i: &[u8]) -> IResult<HashMap<&[u8], &[u8]>> {
    let (i, (_, _, map)) = tuple((command_prefix, tag(b"infoResponse\x0A"), key_value))(i)?;
    Ok((i, map))
}
