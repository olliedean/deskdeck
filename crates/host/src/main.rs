use deskdeck_core::config::deck_from_file_or_create;
use deskdeck_core::input::enigo::EnigoInput;
use deskdeck_server::{create_app_state, router};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let deck = deck_from_file_or_create("deck.toml").expect("Failed to load deck.toml");
    let input = Arc::new(EnigoInput::new().expect("Failed to initialize desktop input"));
    let state = create_app_state(deck, input, PathBuf::from("deck.toml"));

    let bind_addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(bind_addr).await.expect("Failed to bind");

    println!("DeskDeck v0.1.0 running");
    println!("Local:   http://127.0.0.1:8080");
    match network_ipv4_addr() {
        Ok(addr) => {
            println!("Network: http://{}:8080", addr);
            println!("Pairing: http://{}:8080/pair", addr);
        }
        Err(error) => eprintln!("Network: unable to determine LAN address ({})", error),
    }
    println!("Pairing (local): http://127.0.0.1:8080/pair");
    println!("LAN access is unauthenticated; only run DeskDeck on a trusted network.");
    println!("Press any button on the tablet!");

    axum::serve(
        listener,
        router(Arc::clone(&state)).into_make_service_with_connect_info::<SocketAddr>(),
    )
        .await
        .expect("Failed to serve");
}

fn network_ipv4_addr() -> std::io::Result<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect(("1.1.1.1", 80))?;
    match socket.local_addr()?.ip() {
        std::net::IpAddr::V4(addr) if !addr.is_unspecified() && !addr.is_loopback() => Ok(addr),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "no non-loopback IPv4 route is available",
        )),
    }
}
