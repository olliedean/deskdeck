use super::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSize {
    pub columns: u8,
    pub rows: u8,
}

impl GridSize {
    pub fn new(columns: u8, rows: u8) -> Result<Self, DomainError> {
        if columns == 0 || rows == 0 || usize::from(columns) * usize::from(rows) > 80 {
            return Err(DomainError::InvalidGridSize);
        }
        Ok(Self { columns, rows })
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(self) -> usize {
        usize::from(self.columns) * usize::from(self.rows)
    }

    pub fn capacity(self) -> usize {
        self.len()
    }

    pub fn contains(self, position: GridPosition) -> bool {
        position.column < self.columns && position.row < self.rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridPosition {
    pub column: u8,
    pub row: u8,
}

impl GridPosition {
    pub fn new(column: u8, row: u8, grid: GridSize) -> Result<Self, DomainError> {
        if column >= grid.columns || row >= grid.rows {
            return Err(DomainError::PositionOutOfBounds {
                column,
                row,
                columns: grid.columns,
                rows: grid.rows,
            });
        }
        Ok(Self { column, row })
    }

    pub fn from_index(index: usize, grid: GridSize) -> Result<Self, DomainError> {
        if index >= grid.capacity() {
            return Err(DomainError::PositionIndexOutOfBounds {
                position: index,
                capacity: grid.capacity(),
            });
        }
        Ok(Self {
            column: (index % usize::from(grid.columns)) as u8,
            row: (index / usize::from(grid.columns)) as u8,
        })
    }

    pub fn index(self, grid: GridSize) -> Result<usize, DomainError> {
        if grid.contains(self) == false {
            return Err(DomainError::PositionIndexOutOfBounds {
                position: usize::from(self.row) * usize::from(grid.columns)
                    + usize::from(self.column),
                capacity: grid.capacity(),
            });
        }
        Ok(usize::from(self.row) * usize::from(grid.columns) + usize::from(self.column))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Slot {
    Empty,
    Button(super::Button),
}