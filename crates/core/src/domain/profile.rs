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
use std::collections::HashSet;
use std::time::{Duration, SystemTime};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub grid: GridSize,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub pages: Vec<Page>,
    pub default_page_id: PageId,
    pub description: Option<String>,
    pub accent_color: Option<String>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "profile" });
        }
        let grid = GridSize::new(5, 3)?;
        let first_page = Page::new(PageId::new(Uuid::new_v4().to_string())?, "Home", grid)?;
        let default_page_id = first_page.id.clone();
        let now = SystemTime::now();
        let profile = Self {
            id: ProfileId::new(Uuid::new_v4().to_string())?,
            name,
            grid,
            created_at: now,
            updated_at: now,
            pages: vec![first_page],
            default_page_id,
            description: None,
            accent_color: None,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "profile" });
        }
        self.name = name;
        self.touch();
        Ok(())
    }

    pub fn add_page(&mut self, page: Page) -> Result<(), DomainError> {
        self.pages.push(page);
        if let Err(error) = self.validate() {
            self.pages.pop();
            return Err(error);
        }
        self.touch();
        Ok(())
    }

    pub fn remove_page(&mut self, id: &PageId) -> Result<Page, DomainError> {
        let Some(index) = self.pages.iter().position(|page| &page.id == id) else {
            return Err(DomainError::PageNotFound { id: id.clone() });
        };
        if self.pages.len() == 1 {
            return Err(DomainError::EmptyProfile);
        }

        let removed = self.pages.remove(index);
        if self.default_page_id == removed.id {
            self.default_page_id = self.pages[0].id.clone();
        }
        self.touch();
        Ok(removed)
    }

    pub fn duplicate(&self) -> Result<Self, DomainError> {
        self.validate()?;
        let now = SystemTime::now();
        let mut pages = Vec::with_capacity(self.pages.len());

        for page in &self.pages {
            let mut duplicated_page = page.clone();
            duplicated_page.id = PageId::new(Uuid::new_v4().to_string())?;
            duplicated_page.created_at = now;
            duplicated_page.updated_at = now;
            for slot in &mut duplicated_page.slots {
                if let Slot::Button(button) = slot {
                    button.id = ButtonId::new(Uuid::new_v4().to_string())?;
                    button.created_at = now;
                    button.updated_at = now;
                    for action in &mut button.actions {
                        action.id = ActionInstanceId(Uuid::new_v4());
                    }
                }
            }
            pages.push(duplicated_page);
        }

        let default_page_index = self
            .pages
            .iter()
            .position(|page| page.id == self.default_page_id)
            .ok_or(DomainError::InvalidDefaultPage)?;
        let duplicated = Self {
            id: ProfileId::new(Uuid::new_v4().to_string())?,
            name: self.name.clone(),
            grid: self.grid,
            created_at: now,
            updated_at: now,
            default_page_id: pages[default_page_index].id.clone(),
            pages,
            description: self.description.clone(),
            accent_color: self.accent_color.clone(),
        };
        duplicated.validate()?;
        Ok(duplicated)
    }

    pub fn set_default_page(&mut self, id: &PageId) -> Result<(), DomainError> {
        if !self.pages.iter().any(|page| &page.id == id) {
            return Err(DomainError::InvalidDefaultPage);
        }
        self.default_page_id = id.clone();
        self.touch();
        Ok(())
    }

    pub fn page(&self, id: &PageId) -> Result<&Page, DomainError> {
        self.pages
            .iter()
            .find(|page| &page.id == id)
            .ok_or_else(|| DomainError::PageNotFound { id: id.clone() })
    }

    pub fn page_mut(&mut self, id: &PageId) -> Result<&mut Page, DomainError> {
        let Some(index) = self.pages.iter().position(|page| &page.id == id) else {
            return Err(DomainError::PageNotFound { id: id.clone() });
        };
        self.touch();
        self.pages[index].updated_at = SystemTime::now();
        Ok(&mut self.pages[index])
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "profile" });
        }
        if self.pages.is_empty() {
            return Err(DomainError::EmptyProfile);
        }

        let mut page_ids = HashSet::new();
        let mut button_ids = HashSet::new();
        let mut default_page_found = false;
        for page in &self.pages {
            if page.name.trim().is_empty() {
                return Err(DomainError::EmptyName { kind: "page" });
            }
            if page.slots.len() != self.grid.capacity() {
                return Err(DomainError::InvalidSlotCount {
                    expected: self.grid.capacity(),
                    actual: page.slots.len(),
                });
            }
            if !page_ids.insert(&page.id) {
                return Err(DomainError::DuplicatePageId);
            }
            if page.id == self.default_page_id {
                default_page_found = true;
            }
            page.validate()?;
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
                let mut action_ids = Vec::with_capacity(button.actions.len());
                for action in &button.actions {
                    if action_ids.contains(&&action.id) {
                        return Err(DomainError::DuplicateActionId {
                            id: action.id.clone(),
                        });
                    }
                    action_ids.push(&action.id);
                }
            }
        }
        if !default_page_found {
            return Err(DomainError::InvalidDefaultPage);
        }
        Ok(())
    }

    fn touch(&mut self) {
        let now = SystemTime::now();
        self.updated_at = if now > self.updated_at {
            now
        } else {
            self.updated_at + Duration::from_nanos(1)
        };
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
        let default_page_id = pages
            .first()
            .map(|page| page.id.clone())
            .ok_or(DomainError::EmptyProfile)?;
        let now = SystemTime::now();
        let profile = Self {
            id: profile_id,
            name: deck.name,
            grid,
            created_at: now,
            updated_at: now,
            pages,
            default_page_id,
            description: None,
            accent_color: None,
        };
        profile.validate()?;
        Ok(profile)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile() -> Profile {
        Profile::new("Studio").unwrap()
    }

    fn page(profile: &Profile, id: &str, name: &str) -> Page {
        Page::new(PageId::new(id).unwrap(), name, profile.grid).unwrap()
    }

    #[test]
    fn creates_profile_with_default_page() {
        let profile = profile();

        assert_eq!(profile.name, "Studio");
        assert_eq!(profile.pages.len(), 1);
        assert_eq!(profile.pages[0].name, "Home");
        assert_eq!(profile.default_page_id, profile.pages[0].id);
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn rejects_empty_or_whitespace_only_names() {
        for name in ["", " \t\n"] {
            assert!(matches!(
                Profile::new(name),
                Err(DomainError::EmptyName { kind: "profile" })
            ));
        }

        let mut profile = profile();
        assert!(matches!(
            profile.rename("  "),
            Err(DomainError::EmptyName { kind: "profile" })
        ));

        profile.pages[0].name = " \t".to_string();
        assert!(matches!(
            profile.validate(),
            Err(DomainError::EmptyName { kind: "page" })
        ));
    }

    #[test]
    fn adds_and_removes_pages() {
        let mut profile = profile();
        let page = page(&profile, "tools", "Tools");
        let page_id = page.id.clone();

        profile.add_page(page).unwrap();
        assert_eq!(profile.page(&page_id).unwrap().name, "Tools");

        let removed = profile.remove_page(&page_id).unwrap();
        assert_eq!(removed.name, "Tools");
        assert!(matches!(
            profile.page(&page_id),
            Err(DomainError::PageNotFound { .. })
        ));
    }

    #[test]
    fn rejects_removal_of_final_page() {
        let mut profile = profile();
        let page_id = profile.pages[0].id.clone();

        assert!(matches!(
            profile.remove_page(&page_id),
            Err(DomainError::EmptyProfile)
        ));
        assert_eq!(profile.pages.len(), 1);
    }

    #[test]
    fn promotes_first_remaining_page_when_removing_default() {
        let mut profile = profile();
        let home_id = profile.pages[0].id.clone();
        let tools = page(&profile, "tools", "Tools");
        let tools_id = tools.id.clone();
        profile.add_page(tools).unwrap();
        let media = page(&profile, "media", "Media");
        profile.add_page(media).unwrap();
        profile.set_default_page(&tools_id).unwrap();

        profile.remove_page(&tools_id).unwrap();

        assert_eq!(profile.default_page_id, home_id);
    }

    #[test]
    fn rejects_invalid_default_page_id() {
        let mut profile = profile();
        let missing_page_id = PageId::new("missing").unwrap();
        assert!(matches!(
            profile.set_default_page(&missing_page_id),
            Err(DomainError::InvalidDefaultPage)
        ));
        profile.default_page_id = missing_page_id;

        assert!(matches!(
            profile.validate(),
            Err(DomainError::InvalidDefaultPage)
        ));
    }

    #[test]
    fn rejects_duplicate_page_ids() {
        let mut profile = profile();
        profile.pages.push(profile.pages[0].clone());

        assert!(matches!(
            profile.validate(),
            Err(DomainError::DuplicatePageId)
        ));
    }

    #[test]
    fn rejects_duplicate_button_ids_across_pages() {
        let mut profile = profile();
        let button = Button::new(ButtonId::new("shared").unwrap(), "Shared".to_string());
        profile.pages[0].slots[0] = Slot::Button(button.clone());
        let mut other_page = page(&profile, "other", "Other");
        other_page.slots[0] = Slot::Button(button);
        profile.pages.push(other_page);

        assert!(matches!(
            profile.validate(),
            Err(DomainError::DuplicateButtonId)
        ));
    }

    #[test]
    fn rejects_duplicate_action_ids_within_a_button() {
        let mut profile = profile();
        let action = ActionInstance::new("hotkey", json!({})).unwrap();
        let mut button = Button::new(ButtonId::new("home-button").unwrap(), "Home".to_string());
        button.actions = vec![action.clone(), action];
        profile.pages[0].slots[0] = Slot::Button(button);

        assert!(matches!(
            profile.validate(),
            Err(DomainError::DuplicateActionId { .. })
        ));
    }

    #[test]
    fn renames_profile() {
        let mut profile = profile();
        profile.rename("Production").unwrap();

        assert_eq!(profile.name, "Production");
    }

    #[test]
    fn duplicates_profile_with_fresh_ids_and_preserved_page_contents() {
        let mut profile = profile();
        let home_page_id = profile.pages[0].id.clone();
        let mut button = Button::new(ButtonId::new("status").unwrap(), "Status".to_string());
        button.appearance.icon = Some("status-icon".to_string());
        button.appearance.badge_text = Some("Ready".to_string());
        button.content = ButtonContent::Custom {
            markup: "<b>Ready</b>".to_string(),
            css: Some("color: green".to_string()),
        };
        button.state = ButtonState::Pressed;
        button.actions = vec![
            ActionInstance {
                id: ActionInstanceId(Uuid::new_v4()),
                action_type: "first".to_string(),
                settings: json!({"order": 1}),
                enabled: true,
            },
            ActionInstance {
                id: ActionInstanceId(Uuid::new_v4()),
                action_type: "second".to_string(),
                settings: json!({"order": 2}),
                enabled: false,
            },
        ];
        profile.pages[0].slots[3] = Slot::Button(button);
        let mut tools = page(&profile, "tools", "Tools");
        tools.slots[1] = Slot::Button(Button::new(
            ButtonId::new("tools-button").unwrap(),
            "Tools".to_string(),
        ));
        profile.add_page(tools).unwrap();
        profile.description = Some("Production controls".to_string());
        profile.accent_color = Some("#28a745".to_string());

        let duplicate = profile.duplicate().unwrap();

        assert_ne!(duplicate.id, profile.id);
        assert_ne!(duplicate.pages[0].id, profile.pages[0].id);
        assert_ne!(duplicate.pages[1].id, profile.pages[1].id);
        assert_eq!(duplicate.default_page_id, duplicate.pages[0].id);
        assert_eq!(duplicate.description, profile.description);
        assert_eq!(duplicate.accent_color, profile.accent_color);
        assert_eq!(duplicate.pages[0].name, "Home");
        assert_eq!(duplicate.pages[1].name, "Tools");
        assert!(matches!(duplicate.pages[0].slots[0], Slot::Empty));
        let Slot::Button(original_tools_button) = &profile.pages[1].slots[1] else {
            panic!("source tools button should be in slot 1");
        };
        let Slot::Button(duplicated_tools_button) = &duplicate.pages[1].slots[1] else {
            panic!("duplicated tools button should remain in slot 1");
        };
        assert_ne!(duplicated_tools_button.id, original_tools_button.id);
        let Slot::Button(original_button) = &profile.pages[0].slots[3] else {
            panic!("source button should be in slot 3");
        };
        let Slot::Button(duplicated_button) = &duplicate.pages[0].slots[3] else {
            panic!("duplicated button should remain in slot 3");
        };
        assert_ne!(duplicated_button.id, original_button.id);
        assert_eq!(duplicated_button.appearance, original_button.appearance);
        assert_eq!(duplicated_button.content, original_button.content);
        assert_eq!(duplicated_button.state, original_button.state);
        assert_eq!(
            duplicated_button.actions.len(),
            original_button.actions.len()
        );
        for (original_action, duplicated_action) in original_button
            .actions
            .iter()
            .zip(&duplicated_button.actions)
        {
            assert_ne!(duplicated_action.id, original_action.id);
            assert_eq!(duplicated_action.action_type, original_action.action_type);
            assert_eq!(duplicated_action.settings, original_action.settings);
            assert_eq!(duplicated_action.enabled, original_action.enabled);
        }
        assert_ne!(
            profile.page(&home_page_id).unwrap().id,
            duplicate.pages[0].id
        );
    }

    #[test]
    fn mutations_update_profile_timestamp() {
        let mut profile = profile();
        let mut previous = profile.updated_at;

        profile.rename("Renamed").unwrap();
        assert!(profile.updated_at > previous);
        previous = profile.updated_at;

        let second_page = page(&profile, "second", "Second");
        let second_page_id = second_page.id.clone();
        profile.add_page(second_page).unwrap();
        assert!(profile.updated_at > previous);
        previous = profile.updated_at;

        profile.set_default_page(&second_page_id).unwrap();
        assert!(profile.updated_at > previous);
        previous = profile.updated_at;

        profile
            .page_mut(&second_page_id)
            .unwrap()
            .rename("Updated")
            .unwrap();
        assert!(profile.updated_at > previous);
        previous = profile.updated_at;

        profile.remove_page(&second_page_id).unwrap();
        assert!(profile.updated_at > previous);
    }

    #[test]
    fn serializes_and_deserializes_profile() {
        let mut profile = profile();
        profile.description = Some("Desk controls".to_string());
        profile.accent_color = Some("#123456".to_string());
        let serialized = serde_json::to_string(&profile).unwrap();
        let deserialized: Profile = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, profile);
    }
}
