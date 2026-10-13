use axum::{
    extract::{ConnectInfo, Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, Router, routing::get,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::path::PathBuf;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use deskdeck_core::engine::Engine;
use deskdeck_core::config::deck_to_file;
use deskdeck_core::input::InputBackend;
use deskdeck_core::plugins::PluginRegistry;
use deskdeck_protocol::deck::Deck as ProtocolDeck;
use deskdeck_protocol::deck::Page;
use deskdeck_protocol::client::{
    ClientMessage, HostError, HostMessage, Hello, Press, StatePatch, Welcome, ButtonState,
};

pub struct AppState {
    pub engine: Arc<Engine>,
    pub tokens: Arc<RwLock<std::collections::HashMap<String, String>>>,
    pub welcome_tx: tokio::sync::broadcast::Sender<ProtocolDeck>,
    pub deck_path: PathBuf,
}

pub fn create_app_state(
    deck: ProtocolDeck,
    input: Arc<dyn InputBackend>,
    deck_path: PathBuf,
) -> Arc<AppState> {
    let engine = Arc::new(Engine::new(deck.clone(), input));
    let (tx, _rx) = tokio::sync::broadcast::channel(10);
    let _ = tx.send(deck);

    Arc::new(AppState {
        engine,
        tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
        welcome_tx: tx,
        deck_path,
    })
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .route("/pair", get(pairing))
        .route("/pair/{token}", get(pairing_token))
        .route("/deck", get(get_deck).put(put_deck))
        .route("/deck/{page}", get(get_page))
        .route("/actions", get(get_actions))
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
        let mut deck_updates = state.welcome_tx.subscribe();

        let deck = state.engine.deck().await;

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
                msg = deck_updates.recv() => {
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

async fn get_actions(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<deskdeck_core::plugins::ActionDefinition>> {
    Json(state.engine.plugins().definitions())
}

async fn put_deck(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(deck): Json<ProtocolDeck>,
) -> Response {
    if !peer.ip().is_loopback() {
        return (
            StatusCode::FORBIDDEN,
            "deck editing is only allowed from this host; open DeskDeck at http://127.0.0.1:8080",
        )
            .into_response();
    }

    if let Err(error) = validate_deck(&deck, state.engine.plugins()) {
        return (StatusCode::BAD_REQUEST, error).into_response();
    }

    if let Err(error) = deck_to_file(
        state.deck_path.to_string_lossy().as_ref(),
        &deck,
    ) {
        tracing::error!("failed to save deck configuration: {error}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to save deck: {error}"),
        )
            .into_response();
    }

    state.engine.set_deck(deck.clone()).await;
    if let Err(error) = state.welcome_tx.send(deck.clone()) {
        tracing::warn!("deck saved, but no clients received the deck update: {error}");
    }
    Json(deck).into_response()
}

fn validate_deck(deck: &ProtocolDeck, plugins: &PluginRegistry) -> Result<(), String> {
    if deck.name.trim().is_empty() {
        return Err("deck name cannot be empty".to_string());
    }
    if deck.grid.cols == 0 || deck.grid.rows == 0 {
        return Err("deck grid must have at least one column and row".to_string());
    }
    if deck.pages.is_empty() {
        return Err("deck must contain at least one page".to_string());
    }

    let capacity = (usize::from(deck.grid.cols) * usize::from(deck.grid.rows)).min(80);
    let mut ids = std::collections::HashSet::new();
    for page in &deck.pages {
        if page.buttons.len() > capacity {
            return Err(format!(
                "page {:?} has more buttons than the grid can display",
                page.name
            ));
        }
        let mut positions = std::collections::HashSet::new();
        for (index, button) in page.buttons.iter().enumerate() {
            let position = button.position.unwrap_or(index as u8);
            if usize::from(position) >= capacity {
                return Err(format!(
                    "button {:?} has a position outside the grid",
                    button.id
                ));
            }
            if !positions.insert(position) {
                return Err(format!(
                    "page {:?} has multiple buttons in position {}",
                    page.name, position
                ));
            }
            if button.id.trim().is_empty() || button.label.trim().is_empty() {
                return Err("button IDs and labels cannot be empty".to_string());
            }
            if button.background_image.as_ref().is_some_and(|url| {
                url.len() > 2048
                    || (!url.is_empty()
                        && !["http://", "https://", "/", "./", "../"]
                            .iter()
                            .any(|prefix| url.starts_with(prefix)))
            }) {
                return Err(format!(
                    "background image URL is invalid or too long for button {:?}",
                    button.id
                ));
            }
            if button.custom_css.as_ref().is_some_and(|css| {
                css.len() > 8192
                    || ["url(", "@import", "expression(", "javascript:"]
                        .iter()
                        .any(|token| css.to_ascii_lowercase().contains(token))
            }) {
                return Err(format!(
                    "custom CSS is too long or contains disallowed constructs for button {:?}",
                    button.id
                ));
            }
            if button.custom_html.as_ref().is_some_and(|html| html.len() > 32768) {
                return Err(format!("custom HTML is too long for button {:?}", button.id));
            }
            if !ids.insert(button.id.as_str()) {
                return Err(format!("duplicate button ID: {}", button.id));
            }
            for action in &button.on_press {
                plugins
                    .validate(action)
                    .map_err(|error| format!("button {:?}: {error}", button.id))?;
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{create_app_state, router};
    use axum::Json;
    use deskdeck_core::{config::deck_default_home, input::FakeInput};
    use deskdeck_core::plugins::PluginRegistry;
    use deskdeck_protocol::deck::Deck;
    use std::{path::PathBuf, sync::Arc};

    fn validate_deck(deck: &Deck) -> Result<(), String> {
        super::validate_deck(deck, &PluginRegistry::with_defaults())
    }

    #[test]
    fn deck_route_supports_read_and_update_methods() {
        let state = create_app_state(
            deck_default_home(),
            Arc::new(FakeInput::new()),
            PathBuf::from("deck.toml"),
        );
        let _router = router(state);
    }

    #[tokio::test]
    async fn action_catalog_exposes_plugin_field_definitions() {
        let state = create_app_state(
            deck_default_home(),
            Arc::new(FakeInput::new()),
            PathBuf::from("deck.toml"),
        );
        let Json(definitions) = super::get_actions(axum::extract::State(state)).await;
        let hotkey = definitions
            .iter()
            .find(|definition| definition.kind == "hotkey")
            .unwrap();

        assert_eq!(hotkey.fields[0].key, "keys");
        assert_eq!(hotkey.fields[0].label, "Keys (comma-separated)");
    }

    #[test]
    fn rejects_duplicate_button_ids_before_saving() {
        let mut deck = deck_default_home();
        deck.pages[0].buttons[1].id = deck.pages[0].buttons[0].id.clone();

        assert!(validate_deck(&deck).unwrap_err().contains("duplicate button ID"));
    }

    #[test]
    fn rejects_active_css_content_in_custom_styles() {
        let mut deck = deck_default_home();
        deck.pages[0].buttons[0].custom_css = Some("background-image: url(https://attacker.invalid)".into());

        assert!(validate_deck(&deck).unwrap_err().contains("disallowed constructs"));
    }

    #[test]
    fn rejects_duplicate_button_positions() {
        let mut deck = deck_default_home();
        deck.pages[0].buttons[0].position = Some(4);
        deck.pages[0].buttons[1].position = Some(4);

        assert!(validate_deck(&deck).unwrap_err().contains("multiple buttons in position"));
    }

    #[test]
    fn rejects_button_positions_outside_the_grid() {
        let mut deck = deck_default_home();
        deck.pages[0].buttons[0].position = Some(15);

        assert!(validate_deck(&deck).unwrap_err().contains("position outside the grid"));
    }

    #[test]
    fn rejects_invalid_action_configurations() {
        let mut deck = deck_default_home();
        deck.pages[0].buttons[0].on_press[0].config.insert(
            "keys".to_string(),
            serde_json::Value::String("ctrl, ".to_string()),
        );

        assert!(validate_deck(&deck)
            .unwrap_err()
            .contains("hotkey keys must not contain empty entries"));
    }
}