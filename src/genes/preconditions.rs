//! This module deals with evaluating preconditions for gene execution.

use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, VariantArray};

use crate::{
    cells::{Cell, NeighbouringCells},
    genes::*,
};

#[derive(Clone, Debug)]
pub struct PreconditionContext {
    pub neighbouring_organic_energy: NeighbouringEnergy,
    pub neighbouring_charge_energy: NeighbouringEnergy,
    pub neighbouring_cells: NeighbouringCells,
    pub unoccupied_nontoxic_3x3_forward: usize,
    pub unoccupied_nontoxic_3x3_left: usize,
    pub unoccupied_nontoxic_3x3_right: usize,
    pub organism_depth: usize,
    pub cell_energy_has_increased: bool,
    pub has_parent: bool,
    pub rng_value: u8,
}

pub trait Precondition {
    fn evaluate(&self, context: &PreconditionContext) -> bool;
}

/// Conditions related to the resources at the cell's current location.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum CurrentLocationResourceCondition {
    OrganicAtPositionLessThan(f32),
    OrganicAtPositionExceeds(f32),
    ChargeAtPositionLessThan(f32),
    ChargeAtPositionExceeds(f32),
}

impl Precondition for CurrentLocationResourceCondition {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        let organic_energy = &context.neighbouring_organic_energy;
        let charge_energy = &context.neighbouring_charge_energy;

        match self {
            CurrentLocationResourceCondition::OrganicAtPositionLessThan(threshold) => {
                organic_energy.energy_at_center() < *threshold
            }
            CurrentLocationResourceCondition::OrganicAtPositionExceeds(threshold) => {
                organic_energy.energy_at_center() > *threshold
            }
            CurrentLocationResourceCondition::ChargeAtPositionLessThan(threshold) => {
                charge_energy.energy_at_center() < *threshold
            }
            CurrentLocationResourceCondition::ChargeAtPositionExceeds(threshold) => {
                charge_energy.energy_at_center() > *threshold
            }
        }
    }
}

/// Conditions related to the organism depth.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum OrganismDepthCondition {
    IsEven,
    IsOdd,
    GreaterThan(usize),
    LessThan(usize),
}

impl Precondition for OrganismDepthCondition {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        let depth = context.organism_depth;

        match self {
            OrganismDepthCondition::IsEven => depth.is_multiple_of(2),
            OrganismDepthCondition::IsOdd => !depth.is_multiple_of(2),
            OrganismDepthCondition::GreaterThan(threshold) => depth > *threshold,
            OrganismDepthCondition::LessThan(threshold) => depth < *threshold,
        }
    }
}

/// Conditions related to the change in energy of the cell since the last tick.
#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CellEnergyComparison {
    HasIncreased,
    HasDecreased,
}

impl Precondition for CellEnergyComparison {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        match self {
            CellEnergyComparison::HasIncreased => context.cell_energy_has_increased,
            CellEnergyComparison::HasDecreased => !context.cell_energy_has_increased,
        }
    }
}

/// Conditions related to the energy in the soil around the cell, either organic
/// or charge energy, in a 3x3 area centered on the cell.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum SoilEnergyAreaComparison {
    Organic3x3GreaterThanThreshold(f32),
    Organic3x3LessThanThreshold(f32),
    Charge3x3GreaterThanThreshold(f32),
    Charge3x3LessThanThreshold(f32),
    Organic3x3GreaterThanCharge3x3,
    Organic3x3LessThanCharge3x3,
}

impl Precondition for SoilEnergyAreaComparison {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        let organic_energy = &context.neighbouring_organic_energy;
        let charge_energy = &context.neighbouring_charge_energy;

        match self {
            SoilEnergyAreaComparison::Organic3x3GreaterThanThreshold(threshold) => {
                organic_energy.total_energy() > *threshold
            }
            SoilEnergyAreaComparison::Organic3x3LessThanThreshold(threshold) => {
                organic_energy.total_energy() < *threshold
            }
            SoilEnergyAreaComparison::Charge3x3GreaterThanThreshold(threshold) => {
                charge_energy.total_energy() > *threshold
            }
            SoilEnergyAreaComparison::Charge3x3LessThanThreshold(threshold) => {
                charge_energy.total_energy() < *threshold
            }
            SoilEnergyAreaComparison::Organic3x3GreaterThanCharge3x3 => {
                organic_energy.total_energy() > charge_energy.total_energy()
            }
            SoilEnergyAreaComparison::Organic3x3LessThanCharge3x3 => {
                organic_energy.total_energy() < charge_energy.total_energy()
            }
        }
    }
}

/// Conditions related to the cell's awareness of its surroundings.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum SpatialAwarenessCondition {
    NearbyEdibleCells,
    /// Checks if the forward, left, and right neighbouring cells are all unoccupied.
    Empty3Neighbourhood,
    EmptyRelativeDirection(RelativeDirection),
    ObstacleInDirection(RelativeDirection),
    HasParent,
}

impl Precondition for SpatialAwarenessCondition {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        let neighbouring_cells = &context.neighbouring_cells;

        match self {
            SpatialAwarenessCondition::NearbyEdibleCells => neighbouring_cells
                .cells
                .iter()
                .any(|cell| cell.map_or(false, |c| c.is_consumable())),
            SpatialAwarenessCondition::Empty3Neighbourhood => {
                let forward_empty = neighbouring_cells
                    .cell_in_dir(RelativeDirection::Forward)
                    .is_none();
                let left_empty = neighbouring_cells
                    .cell_in_dir(RelativeDirection::Left)
                    .is_none();
                let right_empty = neighbouring_cells
                    .cell_in_dir(RelativeDirection::Right)
                    .is_none();

                forward_empty && left_empty && right_empty
            }
            SpatialAwarenessCondition::EmptyRelativeDirection(relative_direction) => {
                neighbouring_cells
                    .cell_in_dir(*relative_direction)
                    .is_none()
            }
            SpatialAwarenessCondition::ObstacleInDirection(relative_direction) => {
                neighbouring_cells
                    .cell_in_dir(*relative_direction)
                    .is_some()
            }
            SpatialAwarenessCondition::HasParent => context.has_parent,
        }
    }
}

/// Conditions comparing the energy in the soil in a specific directions.
#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DirectionPairComparison {
    ForwardGreaterThanLeft,
    ForwardGreaterThanRight,
    LeftGreaterThanForward,
    LeftGreaterThanRight,
    RightGreaterThanForward,
    RightGreaterThanLeft,
}

impl DirectionPairComparison {
    pub fn compare_energy(&self, energy: &NeighbouringEnergy) -> bool {
        let forward = energy.energy_in_dir(RelativeDirection::Forward);
        let left = energy.energy_in_dir(RelativeDirection::Left);
        let right = energy.energy_in_dir(RelativeDirection::Right);

        match self {
            DirectionPairComparison::ForwardGreaterThanLeft => forward > left,
            DirectionPairComparison::ForwardGreaterThanRight => forward > right,
            DirectionPairComparison::LeftGreaterThanForward => left > forward,
            DirectionPairComparison::LeftGreaterThanRight => left > right,
            DirectionPairComparison::RightGreaterThanForward => right > forward,
            DirectionPairComparison::RightGreaterThanLeft => right > left,
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum DirectionComparison {
    DirectionPairComparison(DirectionPairComparison),
    DirectionGreaterThanThreshold(RelativeDirection, f32),
}

impl DirectionComparison {
    pub fn compare(&self, energy: &NeighbouringEnergy) -> bool {
        match self {
            DirectionComparison::DirectionPairComparison(pair_comparison) => {
                pair_comparison.compare_energy(energy)
            }
            DirectionComparison::DirectionGreaterThanThreshold(relative_direction, threshold) => {
                energy.energy_in_dir(*relative_direction) > *threshold
            }
        }
    }
}

/// Conditions comparing the energy in the soil in a specific direction to a threshold value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrganicEnergyComparison(pub DirectionComparison);

impl Precondition for OrganicEnergyComparison {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        self.0.compare(&context.neighbouring_organic_energy)
    }
}

/// Conditions comparing the energy in the soil in a specific direction to a threshold value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChargeEnergyComparison(pub DirectionComparison);

impl Precondition for ChargeEnergyComparison {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        self.0.compare(&context.neighbouring_charge_energy)
    }
}

/// Conditions comparing the presence of free space and non-toxic soil of the
/// 3x3 region in a specific direction to a threshold value.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum UnoccupiedNonToxic3x3Comparison {
    DirectionComparison3x3(DirectionPairComparison),
    Direction3x3GreaterThanThreshold(RelativeDirection, usize),
}

impl Precondition for UnoccupiedNonToxic3x3Comparison {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        match self {
            UnoccupiedNonToxic3x3Comparison::DirectionComparison3x3(pair_comparison) => {
                let forward = context.unoccupied_nontoxic_3x3_forward;
                let left = context.unoccupied_nontoxic_3x3_left;
                let right = context.unoccupied_nontoxic_3x3_right;

                match pair_comparison {
                    DirectionPairComparison::ForwardGreaterThanLeft => forward > left,
                    DirectionPairComparison::ForwardGreaterThanRight => forward > right,
                    DirectionPairComparison::LeftGreaterThanForward => left > forward,
                    DirectionPairComparison::LeftGreaterThanRight => left > right,
                    DirectionPairComparison::RightGreaterThanForward => right > forward,
                    DirectionPairComparison::RightGreaterThanLeft => right > left,
                }
            }
            UnoccupiedNonToxic3x3Comparison::Direction3x3GreaterThanThreshold(
                relative_direction,
                threshold,
            ) => {
                let value = match relative_direction {
                    RelativeDirection::Forward => context.unoccupied_nontoxic_3x3_forward,
                    RelativeDirection::Left => context.unoccupied_nontoxic_3x3_left,
                    RelativeDirection::Right => context.unoccupied_nontoxic_3x3_right,
                };

                value > *threshold
            }
        }
    }
}

/// Conditions related to the detecting toxic energy levels in the soil.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum PoisonDetection {
    Organic(RelativeDirection),
    Charge(RelativeDirection),
    Any(RelativeDirection),
}

impl Precondition for PoisonDetection {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        let organic_energy = &context.neighbouring_organic_energy;
        let charge_energy = &context.neighbouring_charge_energy;

        match self {
            PoisonDetection::Organic(relative_direction) => {
                organic_energy.is_toxic_in_dir(*relative_direction)
            }
            PoisonDetection::Charge(relative_direction) => {
                charge_energy.is_toxic_in_dir(*relative_direction)
            }
            PoisonDetection::Any(relative_direction) => {
                organic_energy.is_toxic_in_dir(*relative_direction)
                    || charge_energy.is_toxic_in_dir(*relative_direction)
            }
        }
    }
}

/// Preconditions that are checked before executing a genome command.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum GenomePrecondition {
    CurrentLocationResourceCondition(CurrentLocationResourceCondition),
    OrganismDepthCondition(OrganismDepthCondition),
    CellEnergyComparison(CellEnergyComparison),
    SoilEnergyAreaComparison(SoilEnergyAreaComparison),
    SpatialAwarenessCondition(SpatialAwarenessCondition),
    RandomGreaterThan(u8),
    LightEnergyComparison(DirectionComparison),
    OrganicEnergyComparison(OrganicEnergyComparison),
    ChargeEnergyComparison(ChargeEnergyComparison),
    FreeSpaceComparison(UnoccupiedNonToxic3x3Comparison),
    PoisonDetection(PoisonDetection),
}

impl Precondition for GenomePrecondition {
    fn evaluate(&self, context: &PreconditionContext) -> bool {
        match self {
            GenomePrecondition::CurrentLocationResourceCondition(condition) => {
                condition.evaluate(context)
            }
            GenomePrecondition::OrganismDepthCondition(condition) => condition.evaluate(context),
            GenomePrecondition::CellEnergyComparison(condition) => condition.evaluate(context),
            GenomePrecondition::SoilEnergyAreaComparison(condition) => condition.evaluate(context),
            GenomePrecondition::SpatialAwarenessCondition(condition) => condition.evaluate(context),
            GenomePrecondition::RandomGreaterThan(threshold) => context.rng_value > *threshold,
            GenomePrecondition::LightEnergyComparison(direction_comparison) => {
                direction_comparison.compare(&context.neighbouring_charge_energy)
            }
            GenomePrecondition::OrganicEnergyComparison(condition) => condition.evaluate(context),
            GenomePrecondition::ChargeEnergyComparison(condition) => condition.evaluate(context),
            GenomePrecondition::FreeSpaceComparison(condition) => condition.evaluate(context),
            GenomePrecondition::PoisonDetection(condition) => condition.evaluate(context),
        }
    }
}
