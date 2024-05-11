use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, SocketAddrV4, UdpSocket},
};

const MASTER_SERVER_ADDRS: &[&str] = &[
    "master1.forbidden.gg:27950",
    "master1.forbidden.gg:27011",
    "master2.forbidden.gg:27777",
    "master3.forbidden.gg:42863",
    "master1.icy.gg:27950",
    "master1.icy.gg:27011",
    "master2.icy.gg:27777",
    "master3.icy.gg:42863",
    "hl1.steampowered.com:27010",
    "hl2.steampowered.com:27011",
];
const POST_MASTER: usize = 27950;
const POST_MASTER_STEAM: usize = 27011;
const APP_STEAM_ID: usize = 671610;
const MAX_PACKET_LEN: usize = 1400;
const APP_PROTOCOL_VERSION: usize = 26;
const DEFAULT_BASE_GAME: &str = "basewf";
const APPLICATION: &str = "Warfork";

const PREFIX: &[u8] = b"\xFF\xFF\xFF\xFF";

pub struct MasterServer(pub SocketAddr);

impl MasterServer {
    pub fn create_get_servers(
        name: &str,
        protocol_version: &str,
        full: bool,
        empty: bool,
    ) -> Vec<u8> {
        let mut request = PREFIX.to_vec();
        request.extend(b"getservers");
        request.extend(b" ");
        request.extend(name.as_bytes());
        request.extend(b" ");
        request.extend(protocol_version.as_bytes());
        if full {
            request.extend(b" ");
            request.extend(b"empty");
        }
        if empty {
            request.extend(b" ");
            request.extend(b"full");
        }
        request
    }
    pub fn get_servers(
        &self,
        udp: &UdpSocket,
        name: &str,
        protocol_version: &str,
        full: bool,
        empty: bool,
    ) -> io::Result<()> {
        udp.send_to(
            &Self::create_get_servers(name, protocol_version, full, empty),
            self.0,
        )?;
        Ok(())
    }
    pub fn create_get_servers_ext(
        name: &str,
        protocol_version: &str,
        full: bool,
        empty: bool,
    ) -> Vec<u8> {
        let mut request = PREFIX.to_vec();
        request.extend(b"getserversExt");
        request.extend(b" ");
        request.extend(name.as_bytes());
        request.extend(b" ");
        request.extend(protocol_version.as_bytes());
        if full {
            request.extend(b" ");
            request.extend(b"empty");
        }
        if empty {
            request.extend(b" ");
            request.extend(b"full");
        }
        request
    }

    pub fn get_servers_ext(
        &self,
        udp: &UdpSocket,
        name: &str,
        protocol_version: &str,
        full: bool,
        empty: bool,
    ) -> io::Result<()> {
        udp.send_to(
            &Self::create_get_servers_ext(name, protocol_version, full, empty),
            self.0,
        )?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ResponseParseError {
    UnknownResponse,
}
impl std::error::Error for ResponseParseError {}
impl std::fmt::Display for ResponseParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseParseError::UnknownResponse => write!(f, "Unknown response"),
        }
    }
}

pub enum MasterServerResponse {
    Servers(Vec<SocketAddrV4>),
    ServersExt(Vec<SocketAddr>),
}

impl MasterServerResponse {
    pub fn parse(i: &[u8]) -> Result<Self, ResponseParseError> {
        if let Ok((_, a)) = parse::getserversResponse(i) {
            return Ok(MasterServerResponse::Servers(a));
        }
        if let Ok((_, a)) = parse::getserversExtResponse(i) {
            return Ok(MasterServerResponse::ServersExt(a));
        }
        Err(ResponseParseError::UnknownResponse)
    }
    pub fn recv_and_parse(udp: &UdpSocket) -> io::Result<(Self, SocketAddr)> {
        let mut buf = [0; 1024];
        let (len, addr) = udp.recv_from(&mut buf)?;
        let buf = &buf[..len];
        match Self::parse(buf) {
            Ok(o) => Ok((o, addr)),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

pub struct GameServer(pub SocketAddr);

impl GameServer {
    pub fn create_get_info(challenege: &str) -> Vec<u8> {
        assert!(!challenege.contains(['\\', '/', ';', '"', '%']));
        let mut request = PREFIX.to_vec();
        request.extend(b"getinfo");
        request.extend(b" ");
        request.extend(challenege.as_bytes());
        request
    }

    pub fn get_info(&self, udp: &UdpSocket, challenege: &str) -> io::Result<()> {
        udp.send_to(&Self::create_get_info(challenege), self.0)?;
        Ok(())
    }
}

pub enum GameServerResponse {
    Info(HashMap<String, String>),
}

impl GameServerResponse {
    pub fn parse(i: &[u8]) -> Result<Self, ResponseParseError> {
        if let Ok((_, a)) = parse::infoReponse(i) {
            let a = a
                .into_iter()
                .map(|(k, v)| {
                    (
                        String::from_utf8_lossy(k).into_owned(),
                        String::from_utf8_lossy(v).into_owned(),
                    )
                })
                .collect();
            return Ok(GameServerResponse::Info(a));
        }
        Err(ResponseParseError::UnknownResponse)
    }
    pub fn recv_and_parse(udp: &UdpSocket) -> io::Result<(Self, SocketAddr)> {
        let mut buf = [0; 1024];
        let (len, addr) = udp.recv_from(&mut buf)?;
        let buf = &buf[..len];
        match Self::parse(buf) {
            Ok(o) => Ok((o, addr)),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

#[test]
fn do_query() {
    use std::{collections::HashSet, net::ToSocketAddrs, time::Duration};
    let udp = UdpSocket::bind("0.0.0.0:0").unwrap();
    for master_hn in MASTER_SERVER_ADDRS.iter() {
        let master = match master_hn.to_socket_addrs() {
            Ok(mut n) => n.next().unwrap(),
            Err(e) => {
                println! {"{master_hn} {e}"};
                continue;
            }
        };
        let master = MasterServer(master);
        let res = master.get_servers_ext(
            &udp,
            APPLICATION,
            &APP_PROTOCOL_VERSION.to_string(),
            true,
            true,
        );
        match res {
            Ok(_) => println!("{master_hn} success"),
            Err(e) => println!("{master_hn} {e}"),
        }
    }

    udp.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut servers = HashSet::new();
    while let Ok((response, _)) = MasterServerResponse::recv_and_parse(&udp) {
        let MasterServerResponse::ServersExt(found_servers) = response else {
            panic!()
        };
        servers.extend(found_servers);
    }
    for server_socket in servers {
        let server = GameServer(server_socket);
        match server.get_info(&udp, "hello") {
            Ok(_) => println!("{server_socket} good"),
            Err(e) => println!("{server_socket} {e}"),
        }
    }
    while let Ok((response, addr)) = GameServerResponse::recv_and_parse(&udp) {
        let GameServerResponse::Info(info) = response;
        let info_sorted = {
            let mut info = Vec::from_iter(&info);
            info.sort_by(|a, b| a.0.cmp(b.0));
            info
        };
        println!("{addr}");
        let these_keys = ["sv_hostname", "gametype"];
        // for k in these_keys {
        //     let v = clean_rich_text(&info[k]);
        //     println!("    {k} {v}")
        // }
        for (k, v) in info_sorted {
            println!("    {k} {v}");
        }
    }
}

fn clean_rich_text(s: &str) -> String {
    let s = s.replace("^0", "");
    let s = s.replace("^1", "");
    let s = s.replace("^2", "");
    let s = s.replace("^3", "");
    let s = s.replace("^4", "");
    let s = s.replace("^5", "");
    let s = s.replace("^6", "");
    let s = s.replace("^7", "");
    let s = s.replace("^8", "");
    s.replace("^9", "")
}

#[allow(unused)]
fn print_hexes(bytes: &[u8]) {
    bytes.iter().for_each(|byte| print!("{byte:02x}"));
    println!();
}

#[allow(non_snake_case)]
mod parse {
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
}
