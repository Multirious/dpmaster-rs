use dpmaster::{
    client::{Game, Master},
    master_server_commands::{GetServersExtFilter, GetServersFilter},
};
use futures::{FutureExt, StreamExt};
use std::{net::ToSocketAddrs, time::Duration};
use tokio::net::UdpSocket;

const MASTER_SERVER_ADDR: &str = "master1.forbidden.gg:27950";
const APPLICATION: &[u8] = b"Warfork";
const APP_PROTOCOL_VERSION: &[u8] = b"26";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let addr = MASTER_SERVER_ADDR
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let mut master = Master::with_socket(socket, addr);

    let servers = master
        .get_servers(
            Some(APPLICATION),
            APP_PROTOCOL_VERSION,
            &GetServersFilter {
                empty: true,
                ..Default::default()
            },
            Duration::from_secs(1),
        )
        .await?;

    let responses = futures::stream::iter(servers.addresses)
        .map(|server_addr| async move {
            let mut server = Game::new(server_addr.into()).await?;
            match server
                .get_status(b"hello", Duration::from_millis(1000))
                .await
            {
                Ok(response) => {
                    println!("{server_addr}: Ok");
                    return Ok((server_addr, response));
                }
                Err(e) => {
                    println!("{server_addr}: {e}");
                    Err(e)
                }
            }
        })
        .buffer_unordered(16)
        .collect::<Vec<_>>()
        .await;
    let responses = responses
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    for (addr, response) in &responses {
        if response.player_infos.is_empty() {
            continue;
        }
        let mut infos = response.key_values.iter().collect::<Vec<_>>();
        infos.sort_by(|a, b| a.0.cmp(b.0));
        let name = &response.key_values["sv_hostname"];
        let gametype = &response.key_values["gametype"];
        println!(" {}: {}, {}", clean_rich_text(&name[..]), gametype, addr);
        for (k, v) in infos {
            if k == "sv_hostname" || k == "gametype" {
                continue;
            }
            let v = clean_rich_text(v);
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
