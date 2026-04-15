use bevy::ecs::component::Component;
use bevy::reflect::Reflect;
use rand::distr::{Distribution, StandardUniform};
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{EnumDiscriminants, VariantArray};
use thiserror::Error;

use crate::cells::Cell;
use crate::energy::NeighbouringEnergy;

pub const GENOME_SIZE: usize = 52;
pub const GENOME_COMMAND_PROBABILITY: f64 = 0.5;

#[derive(Error, Debug)]
pub enum GenomeError {
    #[error("Invalid GenomeID value: {0}")]
    InvalidGenomeID(usize),
}

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeID(pub usize);

impl Distribution<GenomeID> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeID {
        GenomeID(rng.random_range(0..GENOME_SIZE))
    }
}

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

/// Cell spawn information determining the types of cells that will be spawned
/// as the organism grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeSpawn {
    pub forward_cell_spawn: Option<Cell>,
    pub right_cell_spawn: Option<Cell>,
    pub left_cell_spawn: Option<Cell>,
}

#[inline]
fn random_spawnable_cell_type<R: Rng + ?Sized>(rng: &mut R) -> Option<Cell> {
    // Original spawn rates:
    // 0..=63 => Sprout
    // 64..=75 => Leaf
    // 76..=85 => Antenna
    // 86..=95 => Root
    // 96..=255 => None
    let roll = rng.random_range(0..=255);
    match roll {
        0..=63 => Some(Cell::Sprout),
        64..=75 => Some(Cell::Leaf),
        76..=85 => Some(Cell::Antenna),
        86..=95 => Some(Cell::Root),
        _ => None,
    }
}

impl Distribution<GenomeSpawn> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeSpawn {
        GenomeSpawn {
            forward_cell_spawn: random_spawnable_cell_type(rng),
            right_cell_spawn: random_spawnable_cell_type(rng),
            left_cell_spawn: random_spawnable_cell_type(rng),
        }
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

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum CurrentLocationResourceCondition {
    OrganicAtPositionLessThanThreshold(f32),
    OrganicAtPositionExceedsThreshold(f32),
    ChargeAtPositionLessThanThreshold(f32),
    ChargeAtPositionExceedsThreshold(f32),
}

impl Distribution<CurrentLocationResourceCondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CurrentLocationResourceCondition {
        use CurrentLocationResourceConditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("CurrentLocationResourceCondition variants should not be empty");

        match variant {
            D::OrganicAtPositionLessThanThreshold => {
                CurrentLocationResourceCondition::OrganicAtPositionLessThanThreshold(rng.random())
            }
            D::OrganicAtPositionExceedsThreshold => {
                CurrentLocationResourceCondition::OrganicAtPositionExceedsThreshold(rng.random())
            }
            D::ChargeAtPositionLessThanThreshold => {
                CurrentLocationResourceCondition::ChargeAtPositionLessThanThreshold(rng.random())
            }
            D::ChargeAtPositionExceedsThreshold => {
                CurrentLocationResourceCondition::ChargeAtPositionExceedsThreshold(rng.random())
            }
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum OrganismDepthCondition {
    DepthIsEven,
    DepthIsOdd,
    DepthGreaterThan(usize),
    DepthLessThan(usize),
}

impl Distribution<OrganismDepthCondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OrganismDepthCondition {
        use OrganismDepthConditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("OrganismDepthCondition variants should not be empty");

        match variant {
            D::DepthIsEven => OrganismDepthCondition::DepthIsEven,
            D::DepthIsOdd => OrganismDepthCondition::DepthIsOdd,
            D::DepthGreaterThan => {
                OrganismDepthCondition::DepthGreaterThan(rng.random_range(0..10))
            }
            D::DepthLessThan => OrganismDepthCondition::DepthLessThan(rng.random_range(0..10)),
        }
    }
}

#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum CellEnergyComparison {
    HasIncreased,
    HasDecreased,
}

impl Distribution<CellEnergyComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellEnergyComparison {
        *CellEnergyComparison::VARIANTS
            .choose(rng)
            .expect("CellEnergyComparison variants should not be empty")
    }
}

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

impl Distribution<SoilEnergyAreaComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SoilEnergyAreaComparison {
        use SoilEnergyAreaComparisonDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("SoilEnergyAreaComparison variants should not be empty");

        match variant {
            D::Organic3x3GreaterThanThreshold => {
                SoilEnergyAreaComparison::Organic3x3GreaterThanThreshold(rng.random())
            }
            D::Organic3x3LessThanThreshold => {
                SoilEnergyAreaComparison::Organic3x3LessThanThreshold(rng.random())
            }
            D::Charge3x3GreaterThanThreshold => {
                SoilEnergyAreaComparison::Charge3x3GreaterThanThreshold(rng.random())
            }
            D::Charge3x3LessThanThreshold => {
                SoilEnergyAreaComparison::Charge3x3LessThanThreshold(rng.random())
            }
            D::Organic3x3GreaterThanCharge3x3 => {
                SoilEnergyAreaComparison::Organic3x3GreaterThanCharge3x3
            }
            D::Organic3x3LessThanCharge3x3 => SoilEnergyAreaComparison::Organic3x3LessThanCharge3x3,
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum SpatialAwarenessCondition {
    NearbyEdibleCells,
    Empty3Neighbourhood,
    EmptyRelativeDirection(RelativeDirection),
    ObstacleInDirection(RelativeDirection),
    HasParent,
}

impl Distribution<SpatialAwarenessCondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SpatialAwarenessCondition {
        use SpatialAwarenessConditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("SpatialAwarenessCondition variants should not be empty");

        match variant {
            D::NearbyEdibleCells => SpatialAwarenessCondition::NearbyEdibleCells,
            D::Empty3Neighbourhood => SpatialAwarenessCondition::Empty3Neighbourhood,
            D::EmptyRelativeDirection => {
                SpatialAwarenessCondition::EmptyRelativeDirection(rng.random())
            }
            D::ObstacleInDirection => SpatialAwarenessCondition::ObstacleInDirection(rng.random()),
            D::HasParent => SpatialAwarenessCondition::HasParent,
        }
    }
}

#[derive(VariantArray, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DirectionComparison {
    CenterGreaterThanLeft,
    CenterGreaterThanRight,
    LeftGreaterThanCenter,
    LeftGreaterThanRight,
    RightGreaterThanCenter,
    RightGreaterThanLeft,
}

impl Distribution<DirectionComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> DirectionComparison {
        *DirectionComparison::VARIANTS
            .choose(rng)
            .expect("DirectionComparison variants should not be empty")
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum OrganicEnergyComparison {
    DirectionComparison(DirectionComparison),
    DirectionGreaterThanThreshold(RelativeDirection, f32),
}

impl Distribution<OrganicEnergyComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OrganicEnergyComparison {
        use OrganicEnergyComparisonDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("OrganicEnergyComparison variants should not be empty");

        match variant {
            D::DirectionComparison => OrganicEnergyComparison::DirectionComparison(rng.random()),
            D::DirectionGreaterThanThreshold => {
                OrganicEnergyComparison::DirectionGreaterThanThreshold(
                    rng.random(),
                    rng.random_range(0.0..10.0),
                )
            }
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum ChargeEnergyComparison {
    DirectionComparison(DirectionComparison),
    DirectionGreaterThanThreshold(RelativeDirection, f32),
}

impl Distribution<ChargeEnergyComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> ChargeEnergyComparison {
        use ChargeEnergyComparisonDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("ChargeEnergyComparison variants should not be empty");

        match variant {
            D::DirectionComparison => ChargeEnergyComparison::DirectionComparison(rng.random()),
            D::DirectionGreaterThanThreshold => {
                ChargeEnergyComparison::DirectionGreaterThanThreshold(
                    rng.random(),
                    rng.random_range(0.0..10.0),
                )
            }
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum FreeSpaceComparison {
    DirectionComparison(DirectionComparison),
    DirectionGreaterThanThreshold(RelativeDirection, usize),
}

impl Distribution<FreeSpaceComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FreeSpaceComparison {
        use FreeSpaceComparisonDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("FreeSpaceComparison variants should not be empty");

        match variant {
            D::DirectionComparison => FreeSpaceComparison::DirectionComparison(rng.random()),
            D::DirectionGreaterThanThreshold => FreeSpaceComparison::DirectionGreaterThanThreshold(
                rng.random(),
                rng.random_range(0..10),
            ),
        }
    }
}

#[derive(EnumDiscriminants, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[strum_discriminants(derive(VariantArray))]
pub enum PoisonDetection {
    Organic(RelativeDirection),
    Charge(RelativeDirection),
    Any(RelativeDirection),
}

impl Distribution<PoisonDetection> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PoisonDetection {
        use PoisonDetectionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("PoisonDetection variants should not be empty");

        match variant {
            D::Organic => PoisonDetection::Organic(rng.random()),
            D::Charge => PoisonDetection::Charge(rng.random()),
            D::Any => PoisonDetection::Any(rng.random()),
        }
    }
}

/// Preconditions that are checked before executing a genome command. If the precondition is not met,
/// the genome specified in `GenomeConditional::fail_next_genome` will be executed instead.
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
    FreeSpaceComparison(FreeSpaceComparison),
    PoisonDetection(PoisonDetection),
}

impl GenomePrecondition {
    pub fn check(&self, _params: &PreconditionParameters) -> bool {
        todo!()
    }
}

impl Distribution<GenomePrecondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomePrecondition {
        use GenomePreconditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("GenomePrecondition variants should not be empty");

        match variant {
            D::CurrentLocationResourceCondition => {
                GenomePrecondition::CurrentLocationResourceCondition(rng.random())
            }
            D::OrganismDepthCondition => GenomePrecondition::OrganismDepthCondition(rng.random()),
            D::CellEnergyComparison => GenomePrecondition::CellEnergyComparison(rng.random()),
            D::SoilEnergyAreaComparison => {
                GenomePrecondition::SoilEnergyAreaComparison(rng.random())
            }
            D::SpatialAwarenessCondition => {
                GenomePrecondition::SpatialAwarenessCondition(rng.random())
            }
            D::RandomGreaterThan => GenomePrecondition::RandomGreaterThan(rng.random()),
            D::LightEnergyComparison => GenomePrecondition::LightEnergyComparison(rng.random()),
            D::OrganicEnergyComparison => GenomePrecondition::OrganicEnergyComparison(rng.random()),
            D::ChargeEnergyComparison => GenomePrecondition::ChargeEnergyComparison(rng.random()),
            D::FreeSpaceComparison => GenomePrecondition::FreeSpaceComparison(rng.random()),
            D::PoisonDetection => GenomePrecondition::PoisonDetection(rng.random()),
        }
    }
}

/// Commands that a cell can execute based on its genome.
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

impl Distribution<MultiCellCommand> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MultiCellCommand {
        use MultiCellCommandDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("MultiCellCommand variants should not be empty");

        match variant {
            D::SkipTurn => MultiCellCommand::SkipTurn,
            D::BecomeASeed => MultiCellCommand::BecomeASeed,
            D::BecomeADetachedSeed => MultiCellCommand::BecomeADetachedSeed {
                is_stationary: rng.random_bool(0.5),
            },
            D::Die => MultiCellCommand::Die,
            D::SeparateFromOrganism => MultiCellCommand::SeparateFromOrganism,
            D::TransportSoilEnergy => MultiCellCommand::TransportSoilEnergy(rng.random()),
            D::TransportSoilOrganicMatter => {
                MultiCellCommand::TransportSoilOrganicMatter(rng.random())
            }
            D::ShootSeed => MultiCellCommand::ShootSeed {
                high_energy: rng.random_bool(0.5),
            },
            D::DistributeEnergyAsOrganicMatter => MultiCellCommand::DistributeEnergyAsOrganicMatter,
        }
    }
}

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

impl Distribution<SingleCellCommand> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SingleCellCommand {
        *SingleCellCommand::VARIANTS
            .choose(rng)
            .expect("SingleCellCommand variants should not be empty")
    }
}

/// The next active genome based on the success or failure of executing the main command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeCommandResult {
    pub success_next_genome: GenomeID,
    pub fail_next_genome: GenomeID,
}

impl Distribution<GenomeCommandResult> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeCommandResult {
        GenomeCommandResult {
            success_next_genome: rng.random(),
            fail_next_genome: rng.random(),
        }
    }
}

/// A conditional genome command that checks a precondition and executes either
/// the main command or a fallback genome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeCommands<T> {
    pub preconditions_met_command: Option<T>,
    pub preconditions_unmet_command: Option<T>,
}

impl<T> Distribution<GenomeCommands<T>> for StandardUniform
where
    StandardUniform: Distribution<T>,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeCommands<T> {
        let [preconditions_met_command, preconditions_unmet_command] = std::array::from_fn(|_| {
            rng.random_bool(GENOME_COMMAND_PROBABILITY)
                .then(|| rng.random())
        });

        GenomeCommands {
            preconditions_met_command,
            preconditions_unmet_command,
        }
    }
}

/// The genome of a cell, consisting of spawn information and a set of conditional commands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeEntry {
    pub spawn: GenomeSpawn,
    pub preconditions: Vec<GenomePrecondition>,
    pub multi_cell_commands: GenomeCommands<MultiCellCommand>,
    pub single_cell_commands: GenomeCommands<SingleCellCommand>,
    pub condition_met_fallback: GenomeID,
    pub condition_unmet_fallback: GenomeID,
}

impl Distribution<GenomeEntry> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeEntry {
        GenomeEntry {
            spawn: rng.random(),
            preconditions: (0..rng.random_range(0..2)).map(|_| rng.random()).collect(),
            multi_cell_commands: rng.random(),
            single_cell_commands: rng.random(),
            condition_met_fallback: rng.random(),
            condition_unmet_fallback: rng.random(),
        }
    }
}

#[serde_as]
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    #[serde_as(as = "[_; GENOME_SIZE]")]
    genomes: [GenomeEntry; GENOME_SIZE],
}

impl Genome {
    pub fn get_entry(&self, genome_id: GenomeID) -> &GenomeEntry {
        &self.genomes[genome_id.0]
    }

    pub fn execute(&self, genome_id: &mut GenomeID, precondition_params: &PreconditionParameters) {
        let gene = self.get_entry(*genome_id);

        if gene.preconditions.is_empty() {
            todo!("Spawn cells")
        }

        let all_preconditions_met = gene
            .preconditions
            .iter()
            .all(|precondition| precondition.check(precondition_params));

        let _action = if all_preconditions_met {
            let _multi_command = gene.multi_cell_commands.preconditions_met_command;
            let _single_command = gene.single_cell_commands.preconditions_met_command;

            todo!("Handle multi_command or single_command execution and determine next genome")
        } else {
            let _multi_command = gene.multi_cell_commands.preconditions_unmet_command;
            let _single_command = gene.single_cell_commands.preconditions_unmet_command;

            todo!("Handle multi_command or single_command execution and determine next genome")
        };
    }
}

impl Distribution<Genome> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        Genome {
            genomes: std::array::from_fn(|_| rng.random()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_deserialize_genome() {
        let rng = &mut rand::rng();
        let original_genome: Genome = rng.random();

        let serialized =
            serde_json::to_string(&original_genome).expect("Failed to serialize genome");
        let deserialized: Genome =
            serde_json::from_str(&serialized).expect("Failed to deserialize genome");

        assert_eq!(original_genome, deserialized);
    }
}
