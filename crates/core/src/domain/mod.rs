mod button;
mod error;
mod grid;
mod ids;
mod page;
mod profile;

pub use button::{
    ActionInstance, ActionInstanceId, Border, BorderStyle, Button, ButtonAppearance, ButtonContent,
    ButtonState, HorizontalAlignment, VerticalAlignment,
};
pub use error::DomainError;
pub use grid::{GridPosition, GridSize, Slot};
pub use ids::{ButtonId, PageId, ProfileId};
pub use page::Page;
pub use profile::Profile;

#[cfg(test)]
mod tests {
    use super::{ButtonId, DomainError, GridPosition, GridSize, Profile, Slot};
    use deskdeck_protocol::deck::{Button as ProtocolButton, Deck, Grid};
    use deskdeck_protocol::deck::Page as ProtocolPage;
    use crate::domain::Page;

    #[test]
    fn validates_grid_dimensions_and_positions() {
        assert!(matches!(
            GridSize::new(0, 1),
            Err(DomainError::InvalidGridSize)
        ));
        assert!(matches!(
            GridSize::new(9, 9),
            Err(DomainError::InvalidGridSize)
        ));

        let grid = GridSize::new(5, 3).unwrap();
        let position = GridPosition::from_index(7, grid).unwrap();
        assert_eq!((position.column, position.row), (2, 1));
        assert_eq!(position.index(grid).unwrap(), 7);
        assert!(matches!(
            GridPosition::new(5, 0, grid),
            Err(DomainError::PositionOutOfBounds { .. })
        ));
    }

    #[test]
    fn maps_protocol_buttons_to_stable_grid_slots() {
        let deck = Deck {
            name: "Main".to_string(),
            grid: Grid { cols: 3, rows: 2 },
            pages: vec![ProtocolPage {
                name: "Home".to_string(),
                buttons: vec![ProtocolButton {
                    id: "open".to_string(),
                    label: "Open".to_string(),
                    icon: None,
                    position: Some(4),
                    background_image: None,
                    custom_css: None,
                    custom_html: None,
                    on_press: vec![],
                }],
            }],
        };

        let profile = Profile::try_from(deck).unwrap();
        assert!(matches!(&profile.pages[0].slots[0], Slot::Empty));
        assert!(matches!(&profile.pages[0].slots[4], Slot::Button(_)));
        assert_eq!(profile.to_deck().pages[0].buttons[0].position, Some(4));
    }

    #[test]
    fn preserves_multiple_protocol_pages() {
        let deck = Deck {
            name: "Main".to_string(),
            grid: Grid { cols: 2, rows: 1 },
            pages: vec![
                ProtocolPage {
                    name: "Home".to_string(),
                    buttons: vec![ProtocolButton {
                        id: "home-button".to_string(),
                        label: "Home button".to_string(),
                        icon: None,
                        position: None,
                        background_image: None,
                        custom_css: None,
                        custom_html: None,
                        on_press: vec![],
                    }],
                },
                ProtocolPage {
                    name: "Tools".to_string(),
                    buttons: vec![ProtocolButton {
                        id: "tools-button".to_string(),
                        label: "Tools button".to_string(),
                        icon: None,
                        position: None,
                        background_image: None,
                        custom_css: None,
                        custom_html: None,
                        on_press: vec![],
                    }],
                },
            ],
        };

        let profile = Profile::try_from(deck).unwrap();
        let round_tripped = profile.to_deck();

        assert_eq!(round_tripped.pages.len(), 2);
        assert_eq!(round_tripped.pages[0].name, "Home");
        assert_eq!(round_tripped.pages[0].buttons[0].id, "home-button");
        assert_eq!(round_tripped.pages[1].name, "Tools");
        assert_eq!(round_tripped.pages[1].buttons[0].id, "tools-button");
    }

    #[test]
    fn rejects_invalid_ids_and_page_slot_counts() {
        assert!(matches!(
            ButtonId::new(" "),
            Err(DomainError::InvalidId { kind: "button" })
        ));

        let grid = GridSize::new(2, 2).unwrap();
        let profile = Profile::new(
            super::ProfileId::new("Main").unwrap(),
            "Main",
            grid,
            vec![Page::new(
                super::PageId::new("home").unwrap(),
                "Home".to_string(),
                grid,
            )
            .unwrap()],
        );
        assert!(matches!(profile, Err(DomainError::InvalidSlotCount { .. })));
    }
}