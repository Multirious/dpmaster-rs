use dpmaster::{
    Challenge, GameName, GetServers, GetServersFilter, GetServersResponse, GetStatus,
    ProtocolVersion, StatusResponse,
};
use futures::StreamExt;
use std::{io::Cursor, net::ToSocketAddrs, time::Duration};
use tokio::{net::UdpSocket, select};

const MASTER_SERVER_ADDR: &str = "master1.forbidden.gg:27950";
const APPLICATION: &[u8] = b"Warfork";
const APP_PROTOCOL_VERSION: &[u8] = b"26";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let udp = UdpSocket::bind("0.0.0.0:0").await?;
    let master_socket_addr = MASTER_SERVER_ADDR
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();

    let get_servers = GetServers {
        game_name: Some(GameName::new(APPLICATION.into())?),
        protocol_version: ProtocolVersion::new(APP_PROTOCOL_VERSION.into())?,
        filter: GetServersFilter {
            ..Default::default()
        },
    };

    let mut recv_buf = [0; 1024];
    let mut send_buf = Cursor::new([0; 1024]);
    get_servers.write_all_to(&mut send_buf)?;
    let _ = udp
        .send_to(
            &send_buf.get_ref()[..send_buf.position() as usize],
            master_socket_addr,
        )
        .await?;
    send_buf.set_position(0);

    let mut server_addresses = vec![];
    loop {
        select! {
            read = udp.recv(&mut recv_buf) => {
                let read = read?;
                let recv = &recv_buf[..read];
                let res = match GetServersResponse::parse(recv) {
                    Ok(res) => res,
                    Err(error) => panic!("{error:?}"),
                };
                server_addresses.extend(res.addresses);
                if res.eot {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => break
        }
    }

    let responses = futures::stream::iter(server_addresses)
        .map(|server_addr| async move {
            let udp = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(udp) => udp,
                Err(e) => {
                    println!("Udp bind error: {e}");
                    return None;
                }
            };
            let mut send_buf = Cursor::new([0; 1024]);
            let get_status = GetStatus::new(Challenge::new(b"1234".into()).unwrap());
            match get_status.write_all_to(&mut send_buf) {
                Ok(()) => {}
                Err(_) => {
                    panic!()
                }
            }
            match udp
                .send_to(
                    &send_buf.get_ref()[..send_buf.position() as usize],
                    server_addr,
                )
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    println!("Udp send error: {e}");
                    return None;
                }
            }
            let mut recv_buf = [0; 1024];
            let read = select! {
                read = udp.recv(&mut recv_buf) => {read},
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    println!("Recv timeout error");
                    return None;
                }
            };
            let read = match read {
                Ok(read) => read,
                Err(e) => {
                    println!("Udp recv error: {e}");
                    return None;
                }
            };
            let recv = &recv_buf[..read];
            let status = StatusResponse::parse(recv);
            match status {
                Ok(status) => {
                    println!("{server_addr}: Ok");
                    Some((server_addr, status))
                }
                Err(e) => {
                    println!("{server_addr}: {e:#?}");
                    None
                }
            }
        })
        .buffer_unordered(16)
        .collect::<Vec<_>>()
        .await;
    let responses = responses.into_iter().filter_map(|s| s).collect::<Vec<_>>();

    for (addr, response) in &responses {
        // if response.player_infos.is_empty() {
        //     continue;
        // }
        let mut infos = response.key_values.iter().collect::<Vec<_>>();
        infos.sort_by(|a, b| a.0.cmp(b.0));
        let name = &response.key_values[b"sv_hostname".as_slice()];
        let gametype = &response.key_values[b"gametype".as_slice()];
        println!(
            " {}: {}, {}",
            clean_rich_text(std::str::from_utf8(name).unwrap()),
            std::str::from_utf8(gametype).unwrap(),
            addr
        );
        for (k, v) in infos {
            if k == b"sv_hostname" || k == b"gametype" {
                continue;
            }
            let k = std::str::from_utf8(k).unwrap();
            let v = clean_rich_text(std::str::from_utf8(v).unwrap());
            println!("    {k:>22} {v}")
        }
        println!();
        println!("    PING   FRAGS   TEAM NAME");
        for player in &response.player_infos {
            let name = clean_rich_text(&player.name);
            println!(
                "    {:<6} {:<7} {:<4} {}",
                player.ping, player.frags, player.team, name
            );
        }
        println!();
    }

    Ok(())
}

fn clean_rich_text(s: &str) -> String {
    let to_remove = ["^0", "^1", "^2", "^3", "^4", "^5", "^6", "^7", "^8", "^9"];
    let mut s = s.to_string();
    for c in to_remove {
        s = s.replace(c, "");
    }
    s
}
