use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    Json, Router, routing::get,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use deskdeck_core::engine::Engine;
use deskdeck_core::input::FakeInput;
use deskdeck_protocol::deck::Deck as ProtocolDeck;
use deskdeck_protocol::deck::Page;
use deskdeck_protocol::client::{
    ClientMessage, HostError, HostMessage, Hello, Press, StatePatch, Welcome, ButtonState,
};

pub struct AppState {
    pub engine: Arc<Engine>,
    pub tokens: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub welcome_tx: tokio::sync::broadcast::Sender<ProtocolDeck>,
}

pub fn create_app_state(deck: ProtocolDeck) -> Arc<AppState> {
    let engine = Arc::new(Engine::new(deck.clone(), Arc::new(FakeInput::new())));
    let (tx, _rx) = tokio::sync::broadcast::channel(10);
    let _ = tx.send(deck);

    Arc::new(AppState {
        engine,
        tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
        welcome_tx: tx,
    })
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/pair", get(pairing))
        .route("/pair/:token", get(pairing_token))
        .route("/deck", get(get_deck))
        .route("/deck/:page", get(get_page))
        .fallback_service(ServeDir::new("client/public"))
        .with_state(state)
}

const PROTOCOL_VERSION: u32 = 1;
const HOST_VERSION: &str = "0.1.0";

fn button_states(deck: &ProtocolDeck) -> Vec<ButtonState> {
    deck.pages
        .iter()
        .flat_map(|p| p.buttons.iter())
        .map(|b| ButtonState {
            button_id: b.id.clone(),
            active: false,
        })
        .collect()
}

fn make_welcome(deck: &ProtocolDeck) -> Welcome {
    Welcome {
        host_version: HOST_VERSION.to_string(),
        deck: deck.clone(),
        button_states: button_states(deck),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| async move {
        let (mut tx, mut rx) = socket.split();
        let mut welcome1 = state.welcome_tx.subscribe();
        let mut welcome2 = state.welcome_tx.subscribe();

        // Get initial deck
        let deck = match welcome1.recv().await {
            Ok(deck) => deck,
            Err(_) => ProtocolDeck {
                name: "Default".to_string(),
                grid: deskdeck_protocol::deck::Grid { cols: 5, rows: 3 },
                pages: vec![deskdeck_protocol::deck::Page {
                    name: "Home".to_string(),
                    buttons: vec![deskdeck_protocol::deck::Button {
                        id: "test".to_string(),
                        label: "Test".to_string(),
                        icon: None,
                        on_press: vec![],
                    }],
                }],
            },
        };

        // Send initial welcome
        let welcome_json = serde_json::to_string(&HostMessage::Welcome(make_welcome(&deck))).unwrap();
        if tx.send(axum::extract::ws::Message::Text(welcome_json.into())).await.is_err() {
            return;
        }

        // Handle incoming messages from client and check for broadcast updates
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = rx.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if let axum::extract::ws::Message::Text(text) = msg {
                                match serde_json::from_str::<ClientMessage>(&text) {
                                    Ok(ClientMessage::Hello(Hello { protocol_version, .. })) => {
                                        if protocol_version != PROTOCOL_VERSION {
                                            let _ = tx.send(axum::extract::ws::Message::Text(serde_json::to_string(&HostMessage::Error(HostError {
                                                code: "protocol_mismatch".to_string(),
                                                message: format!("expected v{}, got v{}", PROTOCOL_VERSION, protocol_version),
                                            })).unwrap().into())).await;
                                            break;
                                        }
                                    }
                                    Ok(ClientMessage::Press(Press { button_id, down })) => {
                                        let result = state.engine.press(&button_id, down).await;
                                        let patch = StatePatch::ButtonActive {
                                            button_id: button_id.clone(),
                                            active: down,
                                        };
                                        if tx.send(axum::extract::ws::Message::Text(serde_json::to_string(&HostMessage::StatePatch(patch)).unwrap().into())).await.is_err() {
                                            break;
                                        }

                                        let deck = state.engine.deck().await;
                                        if tx.send(axum::extract::ws::Message::Text(serde_json::to_string(&HostMessage::Welcome(make_welcome(&deck))).unwrap().into())).await.is_err() {
                                            break;
                                        }

                                        if let Err(e) = result {
                                            if tx.send(axum::extract::ws::Message::Text(serde_json::to_string(&HostMessage::Error(HostError {
                                                code: "engine_error".to_string(),
                                                message: format!("{:?}", e),
                                            })).unwrap().into())).await.is_err() {
                                                break;
                                            }
                                        }
                                    }
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                        }
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                // Check for broadcast updates
                msg = welcome2.recv() => {
                    match msg {
                        Ok(deck) => {
                            let msg = serde_json::to_string(&HostMessage::DeckUpdated(deck)).unwrap();
                            if tx.send(axum::extract::ws::Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    })
}

async fn pairing(State(state): State<Arc<AppState>>) -> Response {
    let token = uuid::Uuid::new_v4().to_string();
    state.tokens.write().await.insert(token.clone(), token.clone());
    let qr = qrcode::QrCode::new(&token).unwrap();
    let svg = qr.render::<&str>().build();
    (axum::http::StatusCode::OK, svg).into_response()
}

async fn pairing_token(Path(token): Path<String>, State(state): State<Arc<AppState>>) -> Response {
    let tokens = state.tokens.read().await;
    if tokens.contains_key(&token) {
        (axum::http::StatusCode::OK, "paired").into_response()
    } else {
        (axum::http::StatusCode::UNAUTHORIZED, "invalid token").into_response()
    }
}

async fn get_deck(State(state): State<Arc<AppState>>) -> Json<ProtocolDeck> {
    Json(state.engine.deck().await)
}

async fn get_page(Path(page): Path<String>, State(state): State<Arc<AppState>>) -> Json<Page> {
    let deck = state.engine.deck().await;
    let page = deck
        .pages
        .iter()
        .find(|p| p.name == page)
        .cloned()
        .unwrap_or_else(|| Page {
            name: "Empty".to_string(),
            buttons: vec![],
        });
    Json(page)
}