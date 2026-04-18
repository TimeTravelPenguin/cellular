use bevy::ecs::component::Component;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{EnumDiscriminants, VariantArray};

use crate::{
    cells::Cell,
    genes::preconditions::*,
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

impl IntoIterator for GenomeSpawn {
    type Item = Cell;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut cells = Vec::new();

        if let Some(cell) = self.forward_cell_spawn {
            cells.push(cell);
        }

        if let Some(cell) = self.right_cell_spawn {
            cells.push(cell);
        }

        if let Some(cell) = self.left_cell_spawn {
            cells.push(cell);
        }

        cells.into_iter()
    }
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

#[derive(Clone, Copy, Debug)]
pub enum PreconditionEvaluationResult {
    Unset,
    Met,
    Unmet,
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

    pub fn eval_preconditions(
        &self,
        genome_id: GenomeID,
        context: &PreconditionContext,
    ) -> PreconditionEvaluationResult {
        let entry = self.get_entry(genome_id);
        let preconditions = &entry.preconditions;

        if preconditions.iter().any(|p| p.is_none()) {
            PreconditionEvaluationResult::Unset
        } else if preconditions
            .iter()
            .all(|p| p.as_ref().unwrap().evaluate(context))
        {
            PreconditionEvaluationResult::Met
        } else {
            PreconditionEvaluationResult::Unmet
        }
    }
}
