use super::{
    ActionInstance, Button, ButtonAppearance, ButtonId, ButtonState, DomainError, GridSize, Page,
    PageId, ProfileId, Slot,
};
use deskdeck_protocol::deck::{
    Action as ProtocolAction, Button as ProtocolButton, Deck as ProtocolDeck, Grid as ProtocolGrid,
    Page as ProtocolPage,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub grid: GridSize,
    pub pages: Vec<Page>,
}

impl Profile {
    pub fn new(
        id: ProfileId,
        name: impl Into<String>,
        grid: GridSize,
        pages: Vec<Page>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "profile" });
        }
        if pages.is_empty() {
            return Err(DomainError::EmptyProfile);
        }
        let mut page_ids = std::collections::HashSet::new();
        let mut button_ids = std::collections::HashSet::new();
        for page in &pages {
            if page.name.trim().is_empty() {
                return Err(DomainError::EmptyName { kind: "page" });
            }
            if page.slots.len() != grid.capacity() {
                return Err(DomainError::InvalidSlotCount {
                    expected: grid.capacity(),
                    actual: page.slots.len(),
                });
            }
            if !page_ids.insert(&page.id) {
                return Err(DomainError::DuplicatePageId);
            }
            for button in page.slots.iter().filter_map(|slot| match slot {
                super::Slot::Empty => None,
                super::Slot::Button(button) => Some(button),
            }) {
                if button.label.trim().is_empty() {
                    return Err(DomainError::EmptyName {
                        kind: "button label",
                    });
                }
                if !button_ids.insert(&button.id) {
                    return Err(DomainError::DuplicateButtonId);
                }
            }
        }
        Ok(Self {
            id,
            name,
            grid,
            pages,
        })
    }

    pub fn from_deck(deck: ProtocolDeck) -> Result<Self, DomainError> {
        let grid = GridSize::new(deck.grid.cols, deck.grid.rows)?;
        let profile_id = ProfileId::new(deck.name.clone())?;
        let mut pages = Vec::with_capacity(deck.pages.len());
        for (page_index, protocol_page) in deck.pages.into_iter().enumerate() {
            if protocol_page.buttons.len() > grid.capacity() {
                return Err(DomainError::InvalidSlotCount {
                    expected: grid.capacity(),
                    actual: protocol_page.buttons.len(),
                });
            }
            let mut slots = vec![Slot::Empty; grid.capacity()];
            for (button_index, protocol_button) in protocol_page.buttons.into_iter().enumerate() {
                let position = usize::from(protocol_button.position.unwrap_or(button_index as u8));
                if position >= grid.capacity() {
                    return Err(DomainError::PositionIndexOutOfBounds {
                        position,
                        capacity: grid.capacity(),
                    });
                }
                if !matches!(&slots[position], Slot::Empty) {
                    return Err(DomainError::DuplicatePosition {
                        page: protocol_page.name.clone(),
                        position,
                    });
                }
                slots[position] = Slot::Button(button_from_protocol(protocol_button)?);
            }
            pages.push(Page::new(
                PageId::new(format!("page-{page_index}"))?,
                protocol_page.name,
                grid,
                slots,
            )?);
        }
        Self::new(profile_id, deck.name, grid, pages)
    }

    pub fn to_deck(&self) -> ProtocolDeck {
        ProtocolDeck {
            name: self.name.clone(),
            grid: ProtocolGrid {
                cols: self.grid.columns,
                rows: self.grid.rows,
            },
            pages: self
                .pages
                .iter()
                .map(|page| ProtocolPage {
                    name: page.name.clone(),
                    buttons: page
                        .slots
                        .iter()
                        .enumerate()
                        .filter_map(|(position, slot)| match slot {
                            Slot::Empty => None,
                            Slot::Button(button) => Some(ProtocolButton {
                                id: button.id.as_str().to_string(),
                                label: button.label.clone(),
                                icon: button.appearance.icon.clone(),
                                position: Some(position as u8),
                                background_image: button.appearance.background_image.clone(),
                                custom_css: button.appearance.custom_css.clone(),
                                custom_html: button.appearance.custom_html.clone(),
                                on_press: button.actions.iter().map(Into::into).collect(),
                            }),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl TryFrom<ProtocolDeck> for Profile {
    type Error = DomainError;

    fn try_from(deck: ProtocolDeck) -> Result<Self, Self::Error> {
        Self::from_deck(deck)
    }
}

impl From<ProtocolAction> for ActionInstance {
    fn from(action: ProtocolAction) -> Self {
        Self {
            kind: action.kind,
            config: action.config,
        }
    }
}

impl From<&ActionInstance> for ProtocolAction {
    fn from(action: &ActionInstance) -> Self {
        Self {
            kind: action.kind.clone(),
            config: action.config.clone(),
        }
    }
}

fn button_from_protocol(button: ProtocolButton) -> Result<Button, DomainError> {
    Ok(Button {
        id: ButtonId::new(button.id)?,
        label: button.label,
        appearance: ButtonAppearance {
            icon: button.icon,
            background_image: button.background_image,
            custom_css: button.custom_css,
            custom_html: button.custom_html,
        },
        actions: button.on_press.into_iter().map(Into::into).collect(),
        state: ButtonState::Idle,
    })
}
