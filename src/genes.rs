use bevy::ecs::component::Component;
use bevy::reflect::Reflect;
use rand::distr::{Distribution, StandardUniform};
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{EnumCount, VariantArray};
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
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// Preconditions that are checked before executing a genome command. If the precondition is not met,
/// the genome specified in `GenomeConditional::fail_next_genome` will be executed instead.
#[derive(Component, EnumCount, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum GenomePrecondition {
    CellEnergyHasIncreased,
    SoilEnergyGreaterThanSoilOrganicMatter,
    ObstacleInDirection(RelativeDirection),
    NoObstacles,
    OrganicMatterComparison {
        less_direction: RelativeDirection,
        more_direction: RelativeDirection,
    },
    SoilOrganicMatterMinimumRequirement(f32),
    SoilOrganicMatterMinimumRequirement3x3(f32),
    RngToBeat(u8),
}

impl GenomePrecondition {
    pub fn check(&self, params: &PreconditionParameters) -> bool {
        match self {
            GenomePrecondition::CellEnergyHasIncreased => params.cell_energy_has_increased,
            GenomePrecondition::SoilEnergyGreaterThanSoilOrganicMatter => {
                params.charge_energy.center > params.organic_energy.center
            }
            GenomePrecondition::ObstacleInDirection(dir) => match dir {
                RelativeDirection::Forward => params.obstacles.forward,
                RelativeDirection::Left => params.obstacles.left,
                RelativeDirection::Right => params.obstacles.right,
            },
            GenomePrecondition::NoObstacles => {
                !params.obstacles.forward && !params.obstacles.left && !params.obstacles.right
            }
            GenomePrecondition::OrganicMatterComparison {
                less_direction,
                more_direction,
            } => {
                let less_value = match less_direction {
                    RelativeDirection::Forward => params.organic_energy.forward,
                    RelativeDirection::Left => params.organic_energy.left,
                    RelativeDirection::Right => params.organic_energy.right,
                };

                let more_value = match more_direction {
                    RelativeDirection::Forward => params.organic_energy.forward,
                    RelativeDirection::Left => params.organic_energy.left,
                    RelativeDirection::Right => params.organic_energy.right,
                };

                less_value < more_value
            }
            GenomePrecondition::SoilOrganicMatterMinimumRequirement(min) => {
                params.organic_energy.center >= *min
            }
            GenomePrecondition::SoilOrganicMatterMinimumRequirement3x3(min) => {
                params.organic_energy.total3x3 >= *min
            }
            GenomePrecondition::RngToBeat(threshold) => params.rng_value > *threshold,
        }
    }
}

impl Distribution<GenomePrecondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomePrecondition {
        let total_preconditions = 8;
        debug_assert_eq!(
            GenomePrecondition::COUNT,
            total_preconditions,
            "GenomePrecondition variants should match the number of cases in the distribution"
        );

        match rng.random_range(0..total_preconditions) {
            0 => GenomePrecondition::CellEnergyHasIncreased,
            1 => GenomePrecondition::SoilEnergyGreaterThanSoilOrganicMatter,
            2 => GenomePrecondition::ObstacleInDirection(rng.random()),
            3 => GenomePrecondition::NoObstacles,
            4 => GenomePrecondition::OrganicMatterComparison {
                less_direction: rng.random(),
                more_direction: rng.random(),
            },
            5 => GenomePrecondition::SoilOrganicMatterMinimumRequirement(
                (2 * rng.random_range(0..10)) as f32,
            ),
            6 => GenomePrecondition::SoilOrganicMatterMinimumRequirement3x3(
                (18 * rng.random_range(0..10)) as f32,
            ),
            _ => GenomePrecondition::RngToBeat(rng.random_range(1..u8::MAX)),
        }
    }
}

/// Commands that a cell can execute based on its genome.
#[derive(Component, EnumCount, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        let total_commands = 9;
        debug_assert_eq!(
            MultiCellCommand::COUNT,
            total_commands,
            "MultiCellCommand variants should match the number of cases in the distribution"
        );

        match rng.random_range(0..total_commands) {
            0 => MultiCellCommand::SkipTurn,
            1 => MultiCellCommand::BecomeASeed,
            2 => MultiCellCommand::BecomeADetachedSeed {
                is_stationary: rng.random(),
            },
            3 => MultiCellCommand::Die,
            4 => MultiCellCommand::SeparateFromOrganism,
            5 => MultiCellCommand::TransportSoilEnergy(rng.random()),
            6 => MultiCellCommand::TransportSoilOrganicMatter(rng.random()),
            _ => MultiCellCommand::ShootSeed {
                high_energy: rng.random(),
            },
        }
    }
}

#[derive(Component, VariantArray, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellGenomeCommand {
    // Multi Cell
    pub multi_cell_command: MultiCellCommand,
    pub multi_cell_success_next_genome: GenomeID,
    pub multi_cell_fail_next_genome: GenomeID,
    // Single Cell
    pub single_cell_command: SingleCellCommand,
    pub single_cell_success_next_genome: GenomeID,
    pub single_cell_fail_next_genome: GenomeID,
}

impl Distribution<CellGenomeCommand> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellGenomeCommand {
        CellGenomeCommand {
            multi_cell_command: rng.random(),
            multi_cell_success_next_genome: rng.random(),
            multi_cell_fail_next_genome: rng.random(),
            single_cell_command: rng.random(),
            single_cell_success_next_genome: rng.random(),
            single_cell_fail_next_genome: rng.random(),
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellAction {
    Command(CellGenomeCommand),
    ChangeGenome(GenomeID),
}

impl Distribution<CellAction> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellAction {
        if rng.random_bool(GENOME_COMMAND_PROBABILITY) {
            CellAction::Command(rng.random())
        } else {
            CellAction::ChangeGenome(rng.random())
        }
    }
}

/// A conditional genome command that checks a precondition and executes either
/// the main command or a fallback genome.
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeConditional {
    pub preconditions: Vec<GenomePrecondition>,
    pub preconditions_met_action: CellAction,
    pub preconditions_unmet_action: CellAction,
    pub fallback_genome: GenomeID,
}

impl Distribution<GenomeConditional> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeConditional {
        GenomeConditional {
            preconditions: (0..rng.random_range(0..2)).map(|_| rng.random()).collect(),
            preconditions_met_action: rng.random(),
            preconditions_unmet_action: rng.random(),
            fallback_genome: rng.random(),
        }
    }
}

/// The genome of a cell, consisting of spawn information and a set of conditional commands.
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenomeEntry {
    pub spawn: GenomeSpawn,
    pub conditionals: GenomeConditional,
}

impl Distribution<GenomeEntry> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeEntry {
        GenomeEntry {
            spawn: rng.random(),
            conditionals: rng.random(),
        }
    }
}

#[serde_as]
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    clan: usize,
    #[serde_as(as = "[_; GENOME_SIZE]")]
    genomes: [GenomeEntry; GENOME_SIZE],
}

impl Genome {
    pub fn get_entry(&self, genome_id: GenomeID) -> &GenomeEntry {
        &self.genomes[genome_id.0]
    }

    pub fn execute(
        &self,
        genome_id: &mut GenomeID,
        precondition_params: &PreconditionParameters,
    ) -> CellGenomeCommand {
        let gene = self.get_entry(*genome_id);

        if gene.conditionals.preconditions.is_empty() {
            let next_genome_id = gene.conditionals.fallback_genome;
            *genome_id = next_genome_id;

            return self.execute(genome_id, precondition_params);
        }

        let all_preconditions_met = gene
            .conditionals
            .preconditions
            .iter()
            .all(|precondition| precondition.check(precondition_params));

        let action = if all_preconditions_met {
            gene.conditionals.preconditions_met_action
        } else {
            gene.conditionals.preconditions_unmet_action
        };

        match action {
            CellAction::ChangeGenome(new_genome_id) => {
                *genome_id = new_genome_id;
                self.execute(genome_id, precondition_params)
            }
            CellAction::Command(command) => command,
        }
    }
}

impl Distribution<Genome> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        let genomes = std::array::from_fn(|_| rng.random());

        Genome {
            genomes,
            clan: rng.random_range(0..usize::MAX),
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
