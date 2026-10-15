use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("{kind} ID cannot be empty or contain control characters")]
    InvalidId { kind: &'static str },

    #[error("{kind} name cannot be empty")]
    EmptyName { kind: &'static str },

    #[error("grid dimensions must be non-zero and contain no more than 80 slots")]
    InvalidGridSize,

    #[error("grid position ({column}, {row}) is outside a {columns}x{rows} grid")]
    PositionOutOfBounds {
        column: u8,
        row: u8,
        columns: u8,
        rows: u8,
    },

    #[error("page has {actual} slots, but its grid requires {expected}")]
    InvalidSlotCount { expected: usize, actual: usize },

    #[error("page IDs must be unique")]
    DuplicatePageId,

    #[error("button IDs must be unique within a profile")]
    DuplicateButtonId,

    #[error("action IDs must be unique within a button")]
    DuplicateActionId {
        id: crate::domain::button::ActionInstanceId,
    },

    #[error("page not found: {id}")]
    PageNotFound { id: crate::domain::PageId },

    #[error("button not found: {id}")]
    ButtonNotFound { id: crate::domain::ButtonId },

    #[error("action not found: {id}")]
    ActionNotFound {
        id: crate::domain::button::ActionInstanceId,
    },

    #[error("slot {position} is already occupied")]
    SlotOccupied { position: usize },

    #[error("slot {position} is empty")]
    SlotEmpty { position: usize },

    #[error("button position {position} is outside a grid with {capacity} slots")]
    PositionIndexOutOfBounds { position: usize, capacity: usize },

    #[error("button content exceeds size limit of {limit} bytes")]
    ContentSizeExceeded { limit: usize },

    #[error("a profile must contain at least one page")]
    EmptyProfile,

    #[error("page {page:?} has multiple buttons in slot {position}")]
    DuplicatePosition { page: String, position: usize },

    #[error("invalid default page")]
    InvalidDefaultPage,

    #[error("invalid profile")]
    InvalidProfile,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_variants_exist() {
        let _ = DomainError::InvalidId { kind: "test" };
        let _ = DomainError::EmptyName { kind: "test" };
        let _ = DomainError::InvalidGridSize;
        let _ = DomainError::PositionOutOfBounds {
            column: 0,
            row: 0,
            columns: 10,
            rows: 10,
        };
        let _ = DomainError::InvalidSlotCount {
            expected: 5,
            actual: 3,
        };
        let _ = DomainError::DuplicatePageId;
        let _ = DomainError::DuplicateButtonId;
        let _ = DomainError::DuplicateActionId {
            id: crate::domain::button::ActionInstanceId(::uuid::Uuid::nil()),
        };
        let _ = DomainError::PageNotFound {
            id: crate::domain::PageId::new("test").unwrap(),
        };
        let _ = DomainError::ButtonNotFound {
            id: crate::domain::ButtonId::new("test").unwrap(),
        };
        let _ = DomainError::ActionNotFound {
            id: crate::domain::button::ActionInstanceId(::uuid::Uuid::nil()),
        };
        let _ = DomainError::SlotOccupied { position: 5 };
        let _ = DomainError::SlotEmpty { position: 3 };
        let _ = DomainError::PositionIndexOutOfBounds {
            position: 5,
            capacity: 10,
        };
        let _ = DomainError::ContentSizeExceeded { limit: 100 };
        let _ = DomainError::EmptyProfile;
        let _ = DomainError::DuplicatePosition {
            page: "test".to_string(),
            position: 2,
        };
        let _ = DomainError::InvalidDefaultPage;
        let _ = DomainError::InvalidProfile;
    }

    #[test]
    fn test_error_display_messages() {
        let err = DomainError::InvalidId { kind: "button" };
        assert_eq!(
            format!("{err}"),
            "button ID cannot be empty or contain control characters"
        );

        let err = DomainError::EmptyName { kind: "page" };
        assert_eq!(format!("{err}"), "page name cannot be empty");

        let err = DomainError::InvalidGridSize;
        assert_eq!(
            format!("{err}"),
            "grid dimensions must be non-zero and contain no more than 80 slots"
        );

        let err = DomainError::PositionOutOfBounds {
            column: 5,
            row: 3,
            columns: 10,
            rows: 10,
        };
        assert_eq!(
            format!("{err}"),
            "grid position (5, 3) is outside a 10x10 grid"
        );

        let err = DomainError::InvalidSlotCount {
            expected: 6,
            actual: 4,
        };
        assert_eq!(
            format!("{err}"),
            "page has 4 slots, but its grid requires 6"
        );

        let err = DomainError::DuplicatePageId;
        assert_eq!(format!("{err}"), "page IDs must be unique");

        let err = DomainError::DuplicateButtonId;
        assert_eq!(
            format!("{err}"),
            "button IDs must be unique within a profile"
        );

        let err = DomainError::SlotOccupied { position: 7 };
        assert_eq!(format!("{err}"), "slot 7 is already occupied");

        let err = DomainError::SlotEmpty { position: 2 };
        assert_eq!(format!("{err}"), "slot 2 is empty");

        let err = DomainError::ContentSizeExceeded { limit: 50 };
        assert_eq!(
            format!("{err}"),
            "button content exceeds size limit of 50 bytes"
        );

        let err = DomainError::EmptyProfile;
        assert_eq!(format!("{err}"), "a profile must contain at least one page");

        let err = DomainError::InvalidDefaultPage;
        assert_eq!(format!("{err}"), "invalid default page");

        let err = DomainError::InvalidProfile;
        assert_eq!(format!("{err}"), "invalid profile");
    }
}
