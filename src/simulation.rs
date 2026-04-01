use bevy::{platform::collections::HashMap, prelude::*};
use itertools::Itertools;

use crate::{
    cells::{Cell, FacingDirection},
    genes::RelativeDirection,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellNeighbourPositions {
    pub forward: (usize, usize),
    pub left: (usize, usize),
    pub right: (usize, usize),
}

impl CellNeighbourPositions {
    pub fn new(cell_pos: (usize, usize), facing_direction: FacingDirection) -> Self {
        let (fwd_x, fwd_y) = facing_direction
            .relative(RelativeDirection::Forward)
            .delta();
        let (left_x, left_y) = facing_direction.relative(RelativeDirection::Left).delta();
        let (right_x, right_y) = facing_direction.relative(RelativeDirection::Right).delta();

        Self {
            forward: (
                cell_pos.0.saturating_add_signed(fwd_x),
                cell_pos.1.saturating_add_signed(fwd_y),
            ),
            left: (
                cell_pos.0.saturating_add_signed(left_x),
                cell_pos.1.saturating_add_signed(left_y),
            ),
            right: (
                cell_pos.0.saturating_add_signed(right_x),
                cell_pos.1.saturating_add_signed(right_y),
            ),
        }
    }

    pub fn get(&self, direction: RelativeDirection) -> (usize, usize) {
        match direction {
            RelativeDirection::Forward => self.forward,
            RelativeDirection::Left => self.left,
            RelativeDirection::Right => self.right,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GridBoundary {
    Wrap,
    Fixed,
}

impl GridBoundary {
    pub fn apply(&self, width: usize, height: usize, x: usize, y: usize) -> (usize, usize) {
        match self {
            GridBoundary::Wrap => ((x.rem_euclid(width)), (y.rem_euclid(height))),
            GridBoundary::Fixed => (x.clamp(0, width - 1), y.clamp(0, height - 1)),
        }
    }
}

#[derive(Debug, Clone)]
struct CellNeighbours {
    forward: Cell,
    left: Cell,
    right: Cell,
}

#[derive(Resource, Debug, Clone)]
pub struct SimulationGrid {
    width: usize,
    height: usize,
    boundary: GridBoundary,
    cells: HashMap<(usize, usize), Cell>,
}

impl SimulationGrid {
    pub fn new(width: usize, height: usize, boundary: GridBoundary) -> Self {
        SimulationGrid {
            width,
            height,
            boundary,
            cells: HashMap::default(),
        }
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        self.cells.get(&(x, y))
    }
}
