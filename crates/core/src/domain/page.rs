use super::{ButtonId, DomainError, GridSize, PageId, Slot};

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub id: PageId,
    pub name: String,
    pub slots: Vec<Slot>,
}

impl Page {
    pub fn new(
        id: PageId,
        name: impl Into<String>,
        grid: GridSize,
        slots: Vec<Slot>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName { kind: "page" });
        }
        if slots.len() != grid.capacity() {
            return Err(DomainError::InvalidSlotCount {
                expected: grid.capacity(),
                actual: slots.len(),
            });
        }
        let mut ids = std::collections::HashSet::<&ButtonId>::new();
        for button in slots.iter().filter_map(|slot| match slot {
            Slot::Empty => None,
            Slot::Button(button) => Some(button),
        }) {
            if button.label.trim().is_empty() {
                return Err(DomainError::EmptyName {
                    kind: "button label",
                });
            }
            if !ids.insert(&button.id) {
                return Err(DomainError::DuplicateButtonId);
            }
        }
        Ok(Self { id, name, slots })
    }
}
