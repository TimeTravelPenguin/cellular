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

trait SampleDiscriminant {
    type Output;
    fn sample_discriminant<R: Rng + ?Sized>(&self, rng: &mut R) -> Self::Output;
}

/// Implements `SampleDiscriminant` and `Distribution` for a discriminant/enum pair.
///
/// Supports unit, tuple, and struct variants:
/// ```ignore
/// impl_sample_discriminant! {
///     FooDiscriminants => Foo, |rng| {
///         UnitVariant,
///         TupleVariant(rng.random(), rng.random_range(0..10)),
///         StructVariant { field: rng.random_bool(0.5) },
///     }
/// }
/// ```
macro_rules! impl_sample_discriminant {
    (
        $disc:ident => $target:ident, |$rng:ident| {
            $(
                $variant:ident
                $(( $($field_expr:expr),* $(,)? ))?
                $({ $($field_name:ident : $field_val:expr),* $(,)? })?
            ),*
            $(,)?
        }
    ) => {
        impl SampleDiscriminant for $disc {
            type Output = $target;

            fn sample_discriminant<R: Rng + ?Sized>(&self, $rng: &mut R) -> $target {
                match self {
                    $(
                        Self::$variant => $target::$variant
                            $(( $($field_expr),* ))?
                            $({ $($field_name: $field_val),* })?,
                    )*
                }
            }
        }

        impl Distribution<$target> for StandardUniform {
            fn sample<R: Rng + ?Sized>(&self, $rng: &mut R) -> $target {
                let variant = $disc::VARIANTS
                    .choose($rng)
                    .expect(concat!(stringify!($target), " variants should not be empty"));

                variant.sample_discriminant($rng)
            }
        }
    };
}

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

impl_sample_discriminant! {
    CurrentLocationResourceConditionDiscriminants => CurrentLocationResourceCondition, |rng| {
        OrganicAtPositionLessThan(rng.random()),
        OrganicAtPositionExceeds(rng.random()),
        ChargeAtPositionLessThan(rng.random()),
        ChargeAtPositionExceeds(rng.random()),
    }
}

impl_sample_discriminant! {
    OrganismDepthConditionDiscriminants => OrganismDepthCondition, |rng| {
        IsEven,
        IsOdd,
        GreaterThan(rng.random_range(0..10)),
        LessThan(rng.random_range(0..10)),
    }
}

impl Distribution<CellEnergyComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> CellEnergyComparison {
        *CellEnergyComparison::VARIANTS
            .choose(rng)
            .expect("CellEnergyComparison variants should not be empty")
    }
}

impl_sample_discriminant! {
    SoilEnergyAreaComparisonDiscriminants => SoilEnergyAreaComparison, |rng| {
        Organic3x3GreaterThanThreshold(rng.random()),
        Organic3x3LessThanThreshold(rng.random()),
        Charge3x3GreaterThanThreshold(rng.random()),
        Charge3x3LessThanThreshold(rng.random()),
        Organic3x3GreaterThanCharge3x3,
        Organic3x3LessThanCharge3x3,
    }
}

impl_sample_discriminant! {
    SpatialAwarenessConditionDiscriminants => SpatialAwarenessCondition, |rng| {
        NearbyEdibleCells,
        Empty3Neighbourhood,
        EmptyRelativeDirection(rng.random()),
        ObstacleInDirection(rng.random()),
        HasParent,
    }
}

impl Distribution<DirectionComparison> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> DirectionComparison {
        *DirectionComparison::VARIANTS
            .choose(rng)
            .expect("DirectionComparison variants should not be empty")
    }
}

impl_sample_discriminant! {
    OrganicEnergyComparisonDiscriminants => OrganicEnergyComparison, |rng| {
        DirectionComparison(rng.random()),
        DirectionGreaterThanThreshold(rng.random(), rng.random_range(0.0..10.0)),
    }
}

impl_sample_discriminant! {
    ChargeEnergyComparisonDiscriminants => ChargeEnergyComparison, |rng| {
        DirectionComparison(rng.random()),
        DirectionGreaterThanThreshold(rng.random(), rng.random_range(0.0..10.0)),
    }
}

impl_sample_discriminant! {
    FreeSpaceComparisonDiscriminants => FreeSpaceComparison, |rng| {
        DirectionComparison(rng.random()),
        DirectionGreaterThanThreshold(rng.random(), rng.random_range(0..10)),
    }
}

impl_sample_discriminant! {
    PoisonDetectionDiscriminants => PoisonDetection, |rng| {
        Organic(rng.random()),
        Charge(rng.random()),
        Any(rng.random()),
    }
}

impl_sample_discriminant! {
    GenomePreconditionDiscriminants => GenomePrecondition, |rng| {
        CurrentLocationResourceCondition(rng.random()),
        OrganismDepthCondition(rng.random()),
        CellEnergyComparison(rng.random()),
        SoilEnergyAreaComparison(rng.random()),
        SpatialAwarenessCondition(rng.random()),
        RandomGreaterThan(rng.random()),
        LightEnergyComparison(rng.random()),
        OrganicEnergyComparison(rng.random()),
        ChargeEnergyComparison(rng.random()),
        FreeSpaceComparison(rng.random()),
        PoisonDetection(rng.random()),
    }
}

impl_sample_discriminant! {
    MultiCellCommandDiscriminants => MultiCellCommand, |rng| {
        SkipTurn,
        BecomeASeed,
        BecomeADetachedSeed { is_stationary: rng.random_bool(0.5) },
        Die,
        SeparateFromOrganism,
        TransportSoilEnergy(rng.random()),
        TransportSoilOrganicMatter(rng.random()),
        ShootSeed { high_energy: rng.random_bool(0.5) },
        DistributeEnergyAsOrganicMatter,
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
            preconditions: std::array::from_fn(|_| rng.random::<bool>().then(|| rng.random())),
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
