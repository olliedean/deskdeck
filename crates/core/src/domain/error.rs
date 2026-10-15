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
    #[error("a profile must contain at least one page")]
    EmptyProfile,
    #[error("page {page:?} has multiple buttons in slot {position}")]
    DuplicatePosition { page: String, position: usize },
    #[error("button position {position} is outside a grid with {capacity} slots")]
    PositionIndexOutOfBounds { position: usize, capacity: usize },
}
