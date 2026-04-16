use bevy::ecs::component::Component;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{EnumDiscriminants, VariantArray};

use crate::{
    cells::Cell,
    genes::{GENOME_SIZE, GenomeError, RelativeDirection},
};

/// A unique identifier for a genome entry, represented as an
/// index into the genome.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeID(pub usize);

impl TryFrom<usize> for GenomeID {
    type Error = GenomeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < GENOME_SIZE {
            Ok(GenomeID(value))
        } else {
            Err(GenomeError::InvalidGenomeID(value))
        }
    }
}

/// Cell spawn information determining the types of cells that will be spawned
/// as the organism grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeSpawn {
    pub forward_cell_spawn: Option<Cell>,
    pub right_cell_spawn: Option<Cell>,
    pub left_cell_spawn: Option<Cell>,
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

/// Conditions related to the organism depth.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum OrganismDepthCondition {
    IsEven,
    IsOdd,
    GreaterThan(usize),
    LessThan(usize),
}

/// Conditions related to the change in energy of the cell since the last tick.
#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CellEnergyComparison {
    HasIncreased,
    HasDecreased,
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

/// Conditions comparing the energy in the soil in a specific directions.
#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DirectionComparison {
    CenterGreaterThanLeft,
    CenterGreaterThanRight,
    LeftGreaterThanCenter,
    LeftGreaterThanRight,
    RightGreaterThanCenter,
    RightGreaterThanLeft,
}

/// Conditions comparing the energy in the soil in a specific direction to a threshold value.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum OrganicEnergyComparison {
    DirectionComparison(DirectionComparison),
    DirectionGreaterThanThreshold(RelativeDirection, f32),
}

/// Conditions comparing the energy in the soil in a specific direction to a threshold value.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum ChargeEnergyComparison {
    DirectionComparison(DirectionComparison),
    DirectionGreaterThanThreshold(RelativeDirection, f32),
}

/// Conditions comparing the presence of free space and non-toxic soil of the
/// 3x3 region in a specific direction to a threshold value.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum UnoccupiedNonToxic3x3Comparison {
    DirectionComparison3x3(DirectionComparison),
    Direction3x3GreaterThanThreshold(RelativeDirection, usize),
}

/// Conditions related to the detecting toxic energy levels in the soil.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum PoisonDetection {
    Organic(RelativeDirection),
    Charge(RelativeDirection),
    Any(RelativeDirection),
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

/// Commands that a Sprout with a parent can execute.
#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum MultiCellCommand {
    SkipTurn,
    BecomeASeed,
    BecomeADetachedSeed { is_stationary: bool },
    Die,
    SeparateFromOrganism,
    TransportSoilEnergy(RelativeDirection),
    TransportSoilOrganicMatter(RelativeDirection),
    ShootSeed { high_energy: bool },
    DistributeEnergyAsOrganicMatter,
}

/// Commands that a Sprout without a parent can execute.
#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SingleCellCommand {
    MoveForward,
    TurnLeft,
    TurnRight,
    TurnAround,
    TurnLeftAndMove,
    TurnRightAndMove,
    TurnAroundAndMove,
    TurnRandom,
    MoveRandom,
    Parasitise,
    PullOrganicFromLeft,
    PullOrganicFromRight,
    PullOrganicFromForward,
    PullChargeFromLeft,
    PullChargeFromRight,
    PullChargeFromForward,
    ConsumeNeighbours,
    TakeEnergyFromSoil,
}

/// The next active gene to execute based on the result of executing the current
/// gene's command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeCommandResult {
    pub success_next_genome: GenomeID,
    pub fail_next_genome: GenomeID,
}

/// The commands to execute based on whether the preconditions of a gene are met
/// or not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreconditionCommands<T> {
    pub preconditions_met_command: Option<T>,
    pub preconditions_unmet_command: Option<T>,
}

/// A single entry in the genome of a Sprout.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeEntry {
    pub spawn: GenomeSpawn,
    pub preconditions: [Option<GenomePrecondition>; 2],
    pub multi_cell_commands: PreconditionCommands<MultiCellCommand>,
    pub single_cell_commands: PreconditionCommands<SingleCellCommand>,
    pub condition_met_fallback: GenomeID,
    pub condition_unmet_fallback: GenomeID,
}

/// The complete genome of a Sprout, consisting of a fixed number of genome entries.
#[serde_as]
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    #[serde_as(as = "[_; GENOME_SIZE]")]
    pub genomes: [GenomeEntry; GENOME_SIZE],
}

impl Genome {
    pub fn get_entry(&self, genome_id: GenomeID) -> &GenomeEntry {
        &self.genomes[genome_id.0]
    }
}
