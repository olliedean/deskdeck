use deskdeck_core::config::deck_default_home;
use deskdeck_protocol::deck::Deck;
use deskdeck_server::{create_app_state, router};
use tokio::net::TcpListener;
use tracing_subscriber;
use std::sync::Arc;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let deck = deck_default_home();
    let state = Arc::new(create_app_state(deck));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    println!("DeskDeck v0.1.0 running on http://{}", addr);
    println!("Pairing: http://{}/pair", addr);
    println!("Press any button on the tablet!");

    axum::serve(listener, router(Arc::clone(&state)))
        .await
        .expect("Failed to serve");
}