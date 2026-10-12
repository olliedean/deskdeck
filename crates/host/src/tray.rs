use ksni::menu::{MenuItem, StandardItem};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    OpenDeck,
    OpenPairing,
    OpenConfig,
    Quit,
}

pub struct DeskDeckTray {
    pub actions: UnboundedSender<TrayAction>,
}

impl ksni::Tray for DeskDeckTray {
    fn id(&self) -> String {
        "deskdeck".to_string()
    }

    fn title(&self) -> String {
        "DeskDeck".to_string()
    }

    fn icon_name(&self) -> String {
        "input-gaming".to_string()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let open_deck = self.actions.clone();
        let open_pairing = self.actions.clone();
        let open_config = self.actions.clone();
        let quit = self.actions.clone();

        vec![
            StandardItem {
                label: "Open DeskDeck".to_string(),
                icon_name: "input-gaming".to_string(),
                activate: Box::new(move |_| {
                    queue_action(&open_deck, TrayAction::OpenDeck);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Pair a tablet".to_string(),
                icon_name: "network-wireless".to_string(),
                activate: Box::new(move |_| {
                    queue_action(&open_pairing, TrayAction::OpenPairing);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open deck.toml".to_string(),
                icon_name: "text-x-generic".to_string(),
                activate: Box::new(move |_| {
                    queue_action(&open_config, TrayAction::OpenConfig);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit DeskDeck".to_string(),
                icon_name: "application-exit".to_string(),
                activate: Box::new(move |_| {
                    queue_action(&quit, TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn queue_action(sender: &UnboundedSender<TrayAction>, action: TrayAction) {
    if let Err(error) = sender.send(action) {
        eprintln!("DeskDeck tray action was not delivered: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::DeskDeckTray;
    use tokio::sync::mpsc;

    #[test]
    fn menu_contains_useful_host_actions() {
        let (sender, _receiver) = mpsc::unbounded_channel();
        let tray = DeskDeckTray { actions: sender };
        let menu = ksni::Tray::menu(&tray);

        assert_eq!(menu.len(), 5);
        assert!(matches!(menu[1], ksni::MenuItem::Standard(_)));
        assert!(matches!(menu[3], ksni::MenuItem::Separator));
    }
}
