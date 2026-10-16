use super::{
    ActionInstance, ActionInstanceId, Border, BorderStyle, Button, ButtonAppearance, ButtonContent,
    ButtonId, ButtonState, DomainError, GridSize, HorizontalAlignment, Page, PageId, ProfileId,
    Slot, VerticalAlignment,
};
use deskdeck_protocol::deck::{
    Action as ProtocolAction, Button as ProtocolButton, Deck as ProtocolDeck, Grid as ProtocolGrid,
    Page as ProtocolPage,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AppearanceExtensions {
    font_size: Option<u8>,
    horizontal_alignment: Option<HorizontalAlignment>,
    vertical_alignment: Option<VerticalAlignment>,
    padding: Option<u8>,
    corner_radius: Option<u8>,
    border: Option<Border>,
    opacity: Option<u8>,
    badge_text: Option<String>,
    badge_color: Option<String>,
}

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
                Slot::Empty => None,
                Slot::Button(button) => Some(button),
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
            let mut page = Page::new(
                PageId::new(format!("page-{page_index}"))?,
                protocol_page.name,
                grid,
            )?;
            page.slots = slots;
            pages.push(page);
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
                                background_image: Some(
                                    button
                                        .appearance
                                        .background_color
                                        .clone()
                                        .unwrap_or_default(),
                                ),
                                custom_css: Some(
                                    button.appearance.text_color.clone().unwrap_or_default(),
                                ),
                                custom_html: {
                                    let extensions = AppearanceExtensions {
                                        font_size: button.appearance.font_size,
                                        horizontal_alignment: button
                                            .appearance
                                            .horizontal_alignment
                                            .clone(),
                                        vertical_alignment: button
                                            .appearance
                                            .vertical_alignment
                                            .clone(),
                                        padding: button.appearance.padding,
                                        corner_radius: button.appearance.corner_radius,
                                        border: button.appearance.border.clone(),
                                        opacity: button.appearance.opacity,
                                        badge_text: button.appearance.badge_text.clone(),
                                        badge_color: button.appearance.badge_color.clone(),
                                    };
                                    serde_json::to_string(&extensions).unwrap().into()
                                },
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
        let action_type = if action.kind.trim().is_empty() {
            "unknown"
        } else {
            &action.kind
        };
        ActionInstance::new(action_type, Value::Object(action.config)).unwrap_or_else(|_| {
            ActionInstance {
                id: ActionInstanceId(Uuid::new_v4()),
                action_type: "invalid".to_string(),
                settings: json!({}),
                enabled: false,
            }
        })
    }
}

impl From<&ActionInstance> for ProtocolAction {
    fn from(action: &ActionInstance) -> Self {
        Self {
            kind: action.action_type.clone(),
            config: match &action.settings {
                Value::Object(map) => map.clone(),
                _ => Map::new(),
            },
        }
    }
}

fn button_from_protocol(button: ProtocolButton) -> Result<Button, DomainError> {
    let extensions = match button.custom_html.as_deref() {
        Some(html) => match serde_json::from_str::<AppearanceExtensions>(html) {
            Ok(ext) => ext,
            Err(_) => AppearanceExtensions {
                font_size: None,
                horizontal_alignment: None,
                vertical_alignment: None,
                padding: None,
                corner_radius: None,
                border: None,
                opacity: None,
                badge_text: None,
                badge_color: None,
            },
        },
        None => AppearanceExtensions {
            font_size: None,
            horizontal_alignment: None,
            vertical_alignment: None,
            padding: None,
            corner_radius: None,
            border: None,
            opacity: None,
            badge_text: None,
            badge_color: None,
        },
    };

    Ok(Button {
        id: ButtonId::new(button.id)?,
        label: button.label,
        appearance: ButtonAppearance {
            label: None,
            icon: button.icon,
            background_color: Some(button.background_image.unwrap_or_default()),
            text_color: Some(button.custom_css.unwrap_or_default()),
            font_size: extensions.font_size,
            horizontal_alignment: extensions.horizontal_alignment,
            vertical_alignment: extensions.vertical_alignment,
            padding: extensions.padding,
            corner_radius: extensions.corner_radius,
            border: extensions.border,
            opacity: extensions.opacity,
            badge_text: extensions.badge_text,
            badge_color: extensions.badge_color,
        },
        content: ButtonContent::Basic {
            label: None,
            icon: None,
        },
        actions: button.on_press.into_iter().map(Into::into).collect(),
        state: ButtonState::Default,
        enabled: true,
        created_at: SystemTime::now(),
        updated_at: SystemTime::now(),
    })
}