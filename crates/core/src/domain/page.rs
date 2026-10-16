use super::{Button, ButtonId, DomainError, GridSize, PageId, Slot};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: PageId,
    pub name: String,
    pub grid_size: GridSize,
    pub slots: Vec<Slot>,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

impl Page {
    pub fn new(
        id: PageId,
        name: impl Into<String>,
        grid_size: GridSize,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "page" });
        }

        let slots = vec![Slot::Empty; grid_size.capacity()];
        let now = SystemTime::now();
        Ok(Self {
            id,
            name,
            grid_size,
            slots,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "page" });
        }
        self.name = name;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub fn button_at(&self, position: usize) -> Option<&Button> {
        match self.slots.get(position) {
            Some(Slot::Button(button)) => Some(button),
            _ => None,
        }
    }

    pub fn button_at_mut(&mut self, position: usize) -> Option<&mut Button> {
        match self.slots.get_mut(position) {
            Some(Slot::Button(button)) => Some(button),
            _ => None,
        }
    }

    pub fn add_button(&mut self, button: Button, position: usize) -> Result<(), DomainError> {
        if position >= self.slots.len() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position,
                capacity: self.slots.len(),
            });
        }

        if let Slot::Button(_) = &self.slots[position] {
            return Err(DomainError::SlotOccupied { position });
        }

        for slot in &self.slots {
            if let Slot::Button(existing_button) = slot {
                if existing_button.id == button.id {
                    return Err(DomainError::DuplicateButtonId);
                }
            }
        }

        match &mut self.slots[position] {
            slot @ Slot::Empty => {
                *slot = Slot::Button(button);
                self.updated_at = SystemTime::now();
                Ok(())
            }
            Slot::Button(_) => Err(DomainError::SlotOccupied { position }),
        }
    }

    pub fn remove_button(&mut self, position: usize) -> Result<(), DomainError> {
        if position >= self.slots.len() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position,
                capacity: self.slots.len(),
            });
        }

        match std::mem::replace(&mut self.slots[position], Slot::Empty) {
            Slot::Button(_) => {
                self.updated_at = SystemTime::now();
                Ok(())
            }
            Slot::Empty => Err(DomainError::SlotEmpty { position }),
        }
    }

    pub fn move_button(&mut self, source: usize, destination: usize) -> Result<(), DomainError> {
        if source >= self.slots.len() || destination >= self.slots.len() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position: std::cmp::max(source, destination),
                capacity: self.slots.len(),
            });
        }

        if source != destination {
            if let Slot::Button(existing_button) = &self.slots[destination] {
                if let Slot::Button(source_button) = &self.slots[source] {
                    if existing_button.id == source_button.id {
                        return Err(DomainError::DuplicateButtonId);
                    }
                }
            }
        }

        match (&self.slots[source], &self.slots[destination]) {
            (Slot::Empty, _) => return Err(DomainError::SlotEmpty { position: source }),
            (_, Slot::Empty) => {
                let button = std::mem::replace(&mut self.slots[source], Slot::Empty);
                self.slots[destination] = button;
                self.updated_at = SystemTime::now();
                Ok(())
            }
            (Slot::Button(_), Slot::Button(_)) => {
                let source_button = match std::mem::replace(&mut self.slots[source], Slot::Empty) {
                    Slot::Button(button) => button,
                    Slot::Empty => unreachable!("We just checked this isn't empty"),
                };

                let dest_button = match std::mem::replace(&mut self.slots[destination], Slot::Empty) {
                    Slot::Button(button) => button,
                    Slot::Empty => unreachable!("We just checked this isn't empty"),
                };

                self.slots[source] = Slot::Button(dest_button);
                self.slots[destination] = Slot::Button(source_button);
                self.updated_at = SystemTime::now();
                Ok(())
            }
        }
    }

    pub fn swap_buttons(&mut self, position1: usize, position2: usize) -> Result<(), DomainError> {
        if position1 >= self.slots.len() || position2 >= self.slots.len() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position: std::cmp::max(position1, position2),
                capacity: self.slots.len(),
            });
        }

        match (&self.slots[position1], &self.slots[position2]) {
            (Slot::Empty, _) => return Err(DomainError::SlotEmpty { position: position1 }),
            (_, Slot::Empty) => return Err(DomainError::SlotEmpty { position: position2 }),
            (Slot::Button(_), Slot::Button(_)) => {
                self.slots.swap(position1, position2);
                self.updated_at = SystemTime::now();
                Ok(())
            }
        }
    }

    pub fn duplicate_button(&mut self, source: usize, destination: usize) -> Result<(), DomainError> {
        if source >= self.slots.len() || destination >= self.slots.len() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position: std::cmp::max(source, destination),
                capacity: self.slots.len(),
            });
        }

        if source < destination {
            let (left, right) = self.slots.split_at_mut(destination);
            let source_slot = &mut left[source];
            let dest_slot = &mut right[0];

            let button_to_copy = match source_slot {
                Slot::Button(button) => button,
                Slot::Empty => return Err(DomainError::SlotEmpty { position: source }),
            };

            match dest_slot {
                Slot::Empty => {
                    let mut new_button = button_to_copy.clone();
                    let new_id_string = format!("{}-copy", button_to_copy.id.as_str());
                    new_button.id = ButtonId::new(&new_id_string)
                        .unwrap_or_else(|_| ButtonId::new("copied-button").unwrap());
                    *dest_slot = Slot::Button(new_button);
                    self.updated_at = SystemTime::now();
                    Ok(())
                }
                Slot::Button(_) => Err(DomainError::SlotOccupied { position: destination }),
            }
        } else if source > destination {
            let (left, right) = self.slots.split_at_mut(source);
            let dest_slot = &mut left[destination];
            let source_slot = &mut right[0];

            let button_to_copy = match source_slot {
                Slot::Button(button) => button,
                Slot::Empty => return Err(DomainError::SlotEmpty { position: source }),
            };

            match dest_slot {
                Slot::Empty => {
                    let mut new_button = button_to_copy.clone();
                    let new_id_string = format!("{}-copy", button_to_copy.id.as_str());
                    new_button.id = ButtonId::new(&new_id_string)
                        .unwrap_or_else(|_| ButtonId::new("copied-button").unwrap());
                    *dest_slot = Slot::Button(new_button);
                    self.updated_at = SystemTime::now();
                    Ok(())
                }
                Slot::Button(_) => Err(DomainError::SlotOccupied { position: destination }),
            }
        } else {
            Ok(())
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.slots.len() != self.grid_size.capacity() {
            return Err(DomainError::InvalidSlotCount {
                expected: self.grid_size.capacity(),
                actual: self.slots.len(),
            });
        }

        let mut button_ids = std::collections::HashSet::new();
        for button in self.slots.iter().filter_map(|slot| match slot {
            Slot::Empty => None,
            Slot::Button(button) => Some(button),
        }) {
            if !button_ids.insert(&button.id) {
                return Err(DomainError::DuplicateButtonId);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deskdeck_protocol::deck::Button as ProtocolButton;

    #[test]
    fn test_create_page_with_correct_number_of_slots() {
        let grid_size = GridSize::new(3, 2).unwrap();
        let page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            grid_size,
        )
        .unwrap();

        assert_eq!(page.slots.len(), 6);
        assert!(page.slots.iter().all(|slot| matches!(slot, Slot::Empty)));
    }

    #[test]
    fn test_reject_invalid_grid_sizes() {
        assert!(matches!(
            GridSize::new(0, 5),
            Err(DomainError::InvalidGridSize)
        ));
        assert!(matches!(
            GridSize::new(5, 0),
            Err(DomainError::InvalidGridSize)
        ));
        assert!(matches!(
            GridSize::new(10, 9),
            Err(DomainError::InvalidGridSize)
        ));
    }

    #[test]
    fn test_reject_invalid_positions() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(3, 2).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());
        assert!(matches!(
            page.add_button(button, 10),
            Err(DomainError::PositionIndexOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_add_button() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());
        assert!(page.add_button(button, 0).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(1), None));
    }

    #[test]
    fn test_reject_adding_to_occupied_slot() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("test-button-1").unwrap(), "Test Button 1".to_string());
        let button2 = Button::new(ButtonId::new("test-button-2").unwrap(), "Test Button 2".to_string());

        assert!(page.add_button(button1, 0).is_ok());
        assert!(matches!(
            page.add_button(button2, 0),
            Err(DomainError::SlotOccupied { position: 0 })
        ));
    }

    #[test]
    fn test_reject_adding_duplicate_button_id() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("same-id").unwrap(), "Test Button 1".to_string());
        let button2 = Button::new(ButtonId::new("same-id").unwrap(), "Test Button 2".to_string());

        assert!(page.add_button(button1, 0).is_ok());
        assert!(matches!(
            page.add_button(button2.clone(), 1),
            Err(DomainError::DuplicateButtonId)
        ));
    }

    #[test]
    fn test_remove_button() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());
        assert!(page.add_button(button, 0).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));

        assert!(page.remove_button(0).is_ok());
        assert!(matches!(page.button_at(0), None));
    }

    #[test]
    fn test_reject_removal_from_empty_slot() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            page.remove_button(0),
            Err(DomainError::SlotEmpty { position: 0 })
        ));
    }

    #[test]
    fn test_move_button() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(3, 1).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());
        assert!(page.add_button(button, 0).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), None));

        assert!(page.move_button(0, 2).is_ok());
        assert!(matches!(page.button_at(0), None));
        assert!(matches!(page.button_at(2), Some(_)));
    }

    #[test]
    fn test_reject_move_to_occupied_slot_with_different_id() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(3, 1).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("button1").unwrap(), "Test Button 1".to_string());
        let button2 = Button::new(ButtonId::new("button2").unwrap(), "Test Button 2".to_string());

        assert!(page.add_button(button1, 0).is_ok());
        assert!(page.add_button(button2, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));
        eprintln!("DEBUG: After adds - slot 0: {:?}, slot 2: {:?}", page.button_at(0), page.button_at(2));

        assert!(page.move_button(0, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));
    }

    #[test]
    fn test_reject_move_duplicate_button_id() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(3, 1).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("same-id").unwrap(), "Test Button 1".to_string());
        let button2 = Button::new(ButtonId::new("same-id").unwrap(), "Test Button 2".to_string());

        assert!(page.add_button(button1, 0).is_ok());
        let button2_different_id = Button::new(ButtonId::new("different-id").unwrap(), "Test Button 2".to_string());
        assert!(page.add_button(button2_different_id, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));

        assert!(page.move_button(0, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));
    }

    #[test]
    fn test_swap_buttons() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(3, 1).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("test-button-1").unwrap(), "Test Button 1".to_string());
        let button2 = Button::new(ButtonId::new("test-button-2").unwrap(), "Test Button 2".to_string());

        assert!(page.add_button(button1, 0).is_ok());
        assert!(page.add_button(button2, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));

        assert!(page.swap_buttons(0, 2).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(2), Some(_)));
    }

    #[test]
    fn test_duplicate_button() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let original_button = Button::new(ButtonId::new("original-button").unwrap(), "Original Button".to_string());
        assert!(page.add_button(original_button, 0).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(1), None));

        assert!(page.duplicate_button(0, 1).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(1), Some(_)));
    }

    #[test]
    fn test_duplicate_button_creates_new_id() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let original_button = Button::new(ButtonId::new("original-button").unwrap(), "Original Button".to_string());
        assert!(page.add_button(original_button.clone(), 0).is_ok());
        let original_id = original_button.id.clone();

        assert!(page.duplicate_button(0, 1).is_ok());
        assert!(matches!(page.button_at(0), Some(_)));
        assert!(matches!(page.button_at(1), Some(_)));

        let duplicated_button = page.button_at(1).unwrap();
        assert!(duplicated_button.id != original_id);
    }

    #[test]
    fn test_validate_page() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button1 = Button::new(ButtonId::new("button1").unwrap(), "Button 1".to_string());
        let button2 = Button::new(ButtonId::new("button2").unwrap(), "Button 2".to_string());
        assert!(page.add_button(button1.clone(), 0).is_ok());
        assert!(page.add_button(button2.clone(), 1).is_ok());
        assert!(page.validate().is_ok());

        let mut page_with_duplicates = Page::new(
            PageId::new("test-page-dup").unwrap(),
            "Test Page Dup".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("same-id").unwrap(), "Same ID Button".to_string());
        assert!(page_with_duplicates.add_button(button.clone(), 0).is_ok());
        assert!(matches!(
            page_with_duplicates.add_button(button.clone(), 1),
            Err(DomainError::DuplicateButtonId)
        ));
    }

    #[test]
    fn test_serialisation_round_trip() {
        let original_page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());
        let mut page_with_button = original_page.clone();
        assert!(page_with_button.add_button(button.clone(), 0).is_ok());

        let serialized = serde_json::to_string(&page_with_button).unwrap();

        let deserialized_page: Page = serde_json::from_str(&serialized).unwrap();

        assert_eq!(page_with_button, deserialized_page);
    }

    #[test]
    fn test_timestamp_updates_after_mutation() {
        let mut page = Page::new(
            PageId::new("test-page").unwrap(),
            "Test Page".to_string(),
            GridSize::new(2, 2).unwrap(),
        )
        .unwrap();

        let original_updated_at = page.updated_at;
        let button = Button::new(ButtonId::new("test-button").unwrap(), "Test Button".to_string());

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(page.add_button(button.clone(), 0).is_ok());
        assert!(page.updated_at > original_updated_at);

        let updated_after_add = page.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(page.remove_button(0).is_ok());
        assert!(page.updated_at > updated_after_add);
    }
}