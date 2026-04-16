use rand::Rng;
use rand::distr::{Distribution, StandardUniform};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use strum::VariantArray;
use thiserror::Error;

mod genome;
mod random;

pub use genome::{
    CellEnergyComparison, ChargeEnergyComparison, ChargeEnergyComparisonDiscriminants,
    CurrentLocationResourceCondition, CurrentLocationResourceConditionDiscriminants,
    DirectionComparison, Genome, GenomeCommandResult, GenomeEntry, GenomeID, GenomePrecondition,
    GenomePreconditionDiscriminants, GenomeSpawn, MultiCellCommand, MultiCellCommandDiscriminants,
    OrganicEnergyComparison, OrganicEnergyComparisonDiscriminants, OrganismDepthCondition,
    OrganismDepthConditionDiscriminants, PoisonDetection, PoisonDetectionDiscriminants,
    PreconditionCommands, SingleCellCommand, SoilEnergyAreaComparison,
    SoilEnergyAreaComparisonDiscriminants, SpatialAwarenessCondition,
    SpatialAwarenessConditionDiscriminants, UnoccupiedNonToxic3x3Comparison,
    UnoccupiedNonToxic3x3ComparisonDiscriminants,
};

use crate::energy::NeighbouringEnergy;

pub const GENOME_SIZE: usize = 52;
pub const GENOME_COMMAND_PROBABILITY: f64 = 0.5;

pub trait Mutate {
    fn mutate<R: Rng + ?Sized>(&mut self, rng: &mut R);
}

#[derive(Error, Debug)]
pub enum GenomeError {
    #[error("Invalid GenomeID value: {0}")]
    InvalidGenomeID(usize),
}

/// Directions relative to the cell's facing direction.
#[derive(Clone, Copy, VariantArray, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelativeDirection {
    Left,
    Right,
    Forward,
}

impl Distribution<RelativeDirection> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> RelativeDirection {
        *RelativeDirection::VARIANTS
            .choose(rng)
            .expect("RelativeDirection variants should not be empty")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObstacleInfo {
    pub forward: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreconditionParameters {
    pub organic_energy: NeighbouringEnergy,
    pub charge_energy: NeighbouringEnergy,
    pub cell_energy_has_increased: bool,
    pub obstacles: ObstacleInfo,
    pub rng_value: u8,
}

#[cfg(test)]
mod tests {
    use rand::RngExt;

    use super::*;

    #[test]
    fn serialize_deserialize_genome_equal() {
        let rng = &mut rand::rng();
        let original_genome: Genome = rng.random();

        let serialized =
            serde_json::to_string(&original_genome).expect("Failed to serialize genome");

        let deserialized: Genome =
            serde_json::from_str(&serialized).expect("Failed to deserialize genome");

        assert_eq!(original_genome, deserialized);
    }
}
