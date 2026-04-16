use rand::{
    Rng, RngExt,
    distr::{Distribution, StandardUniform},
};
use strum::VariantArray;

use crate::genes::genome::PreconditionCommands;

use super::{
    Cell, CellEnergyComparison, ChargeEnergyComparison, ChargeEnergyComparisonDiscriminants,
    CurrentLocationResourceCondition, CurrentLocationResourceConditionDiscriminants,
    DirectionComparison, FreeSpaceComparison, FreeSpaceComparisonDiscriminants,
    GENOME_COMMAND_PROBABILITY, GENOME_SIZE, Genome, GenomeCommandResult, GenomeEntry, GenomeID,
    GenomePrecondition, GenomePreconditionDiscriminants, GenomeSpawn, IndexedRandom,
    MultiCellCommand, MultiCellCommandDiscriminants, OrganicEnergyComparison,
    OrganicEnergyComparisonDiscriminants, OrganismDepthCondition,
    OrganismDepthConditionDiscriminants, PoisonDetection, PoisonDetectionDiscriminants,
    SingleCellCommand, SoilEnergyAreaComparison, SoilEnergyAreaComparisonDiscriminants,
    SpatialAwarenessCondition, SpatialAwarenessConditionDiscriminants,
};

impl Distribution<GenomeID> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeID {
        GenomeID(rng.random_range(0..GENOME_SIZE))
    }
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
        0..=63 => Some(Cell::Sprout),   // 25%
        64..=75 => Some(Cell::Leaf),    // 4.7%
        76..=85 => Some(Cell::Antenna), // 3.9%
        86..=95 => Some(Cell::Root),    // 3.9%
        _ => None,                      // 62.5%
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

impl Distribution<CurrentLocationResourceCondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CurrentLocationResourceCondition {
        use CurrentLocationResourceConditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("CurrentLocationResourceCondition variants should not be empty");

        match variant {
            D::OrganicAtPositionLessThan => {
                CurrentLocationResourceCondition::OrganicAtPositionLessThan(rng.random())
            }
            D::OrganicAtPositionExceeds => {
                CurrentLocationResourceCondition::OrganicAtPositionExceeds(rng.random())
            }
            D::ChargeAtPositionLessThan => {
                CurrentLocationResourceCondition::ChargeAtPositionLessThan(rng.random())
            }
            D::ChargeAtPositionExceeds => {
                CurrentLocationResourceCondition::ChargeAtPositionExceeds(rng.random())
            }
        }
    }
}

impl Distribution<OrganismDepthCondition> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> OrganismDepthCondition {
        use OrganismDepthConditionDiscriminants as D;

        let variant = D::VARIANTS
            .choose(rng)
            .expect("OrganismDepthCondition variants should not be empty");

        match variant {
            D::IsEven => OrganismDepthCondition::IsEven,
            D::IsOdd => OrganismDepthCondition::IsOdd,
            D::GreaterThan => OrganismDepthCondition::GreaterThan(rng.random_range(0..10)),
            D::LessThan => OrganismDepthCondition::LessThan(rng.random_range(0..10)),
        }
    }
}

impl Distribution<CellEnergyComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellEnergyComparison {
        *CellEnergyComparison::VARIANTS
            .choose(rng)
            .expect("CellEnergyComparison variants should not be empty")
    }
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

impl Distribution<DirectionComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> DirectionComparison {
        *DirectionComparison::VARIANTS
            .choose(rng)
            .expect("DirectionComparison variants should not be empty")
    }
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

impl Distribution<SingleCellCommand> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SingleCellCommand {
        *SingleCellCommand::VARIANTS
            .choose(rng)
            .expect("SingleCellCommand variants should not be empty")
    }
}

impl Distribution<GenomeCommandResult> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> GenomeCommandResult {
        GenomeCommandResult {
            success_next_genome: rng.random(),
            fail_next_genome: rng.random(),
        }
    }
}

impl<T> Distribution<PreconditionCommands<T>> for StandardUniform
where
    StandardUniform: Distribution<T>,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PreconditionCommands<T> {
        PreconditionCommands {
            preconditions_met_command: rng
                .random_bool(GENOME_COMMAND_PROBABILITY)
                .then(|| rng.random()),
            preconditions_unmet_command: rng
                .random_bool(GENOME_COMMAND_PROBABILITY)
                .then(|| rng.random()),
        }
    }
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

impl Distribution<Genome> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Genome {
        Genome {
            genomes: std::array::from_fn(|_| rng.random()),
        }
    }
}
