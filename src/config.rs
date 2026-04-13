use bevy::{ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

/// Configuration parameters for the simulation.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SimulationParameters {
    /// Height of the simulation grid
    pub height: usize,
    /// Width of the simulation grid
    pub width: usize,
    /// Initial simulation speed in ticks per second
    pub tick_rate: u32,
    /// Initial number of Sprout cells in the simulation
    pub initial_sprout_count: usize,
}

impl Default for SimulationParameters {
    fn default() -> Self {
        Self {
            height: 100,
            width: 100,
            tick_rate: 10,
            // Start with 10% of the grid filled with Sprout cells
            initial_sprout_count: 100 * 100 / 10,
        }
    }
}

/// Configuration parameters for energy extraction rates for different cell types.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ExtractionRateConfig {
    /// Root organic energy extraction per tick
    pub root_extract_rate: f32,
    /// Antennae charge energy extraction per tick
    pub antenna_extract_rate: f32,
    /// Lone Sprout soil absorption per tick
    pub lone_sprout_extract_rate: f32,
}

impl Default for ExtractionRateConfig {
    fn default() -> Self {
        Self {
            root_extract_rate: 1.0,
            antenna_extract_rate: 1.0,
            lone_sprout_extract_rate: 6.0,
        }
    }
}

/// Configuration parameters for toxicity thresholds for organic and charge energy.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToxicityThresholdConfig {
    /// Toxic threshold for organic energy
    pub organic_toxic_threshold: f32,
    /// Toxic threshold for charge energy
    pub charge_toxic_threshold: f32,
}

impl Default for ToxicityThresholdConfig {
    fn default() -> Self {
        Self {
            organic_toxic_threshold: 512.0,
            charge_toxic_threshold: 512.0,
        }
    }
}

/// Configuration parameters for energy costs to keep different cell types alive per tick.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellLivingCostConfig {
    /// Energy cost per tick for Leaf cells
    pub leaf_living_cost: f32,
    /// Energy cost per tick for Root cells
    pub root_living_cost: f32,
    /// Energy cost per tick for Antenna cells
    pub antenna_living_cost: f32,
    /// Energy cost per tick for Branch cells
    pub branch_living_cost: f32,
    /// Energy cost per tick for Sprout cells
    pub sprout_living_cost: f32,
    /// Energy cost per tick for Seed cells
    pub seed_living_cost: f32,
}

impl Default for CellLivingCostConfig {
    fn default() -> Self {
        Self {
            leaf_living_cost: 0.04,
            root_living_cost: 0.04,
            antenna_living_cost: 0.04,
            branch_living_cost: 0.04,
            sprout_living_cost: 1.0,
            seed_living_cost: 0.5,
        }
    }
}

/// Configuration parameters for energy redistribution upon cell death, specifying
/// how much organic energy is released for each cell type when it dies.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellDeathEnergyRedistributionConfig {
    pub leaf_organic_death_energy: f32,
    pub root_organic_death_energy: f32,
    pub antenna_organic_death_energy: f32,
    pub branch_organic_death_energy: f32,
    pub sprout_organic_death_energy: f32,
    pub seed_organic_death_energy: f32,
}

impl Default for CellDeathEnergyRedistributionConfig {
    fn default() -> Self {
        Self {
            leaf_organic_death_energy: 15.0,
            root_organic_death_energy: 15.0,
            antenna_organic_death_energy: 15.0,
            branch_organic_death_energy: 15.0,
            sprout_organic_death_energy: 15.0,
            seed_organic_death_energy: 15.0,
        }
    }
}

/// Configuration parameters for energy costs of cell actions like moving and reproducing.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellActionCostConfig {
    /// Energy cost for moving a Sprout cell
    pub sprout_move_cost: f32,
    /// Energy cost to create a new cell
    pub reproduce_cost: f32,
    /// Energy threshold for Sprout cells to detach and create a new plant
    pub sprout_detach_cost: f32,
    /// Energy threshold for Seed cells to detach and create a new plant
    pub seed_detach_cost: f32,
}

impl Default for CellActionCostConfig {
    fn default() -> Self {
        Self {
            sprout_move_cost: 1.0,
            reproduce_cost: 5.0,
            sprout_detach_cost: 1024.0,
            seed_detach_cost: 512.0,
        }
    }
}

/// Configuration parameters for the environment.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Base light energy factor for Leaf cells
    pub light_energy: f32,
    /// Coefficient for calculating light energy absorption by Leaf cells
    pub light_coef: f32,
    /// Initial organic energy in the environment
    pub initial_organic_energy: f32,
    /// Initial charge energy in the environment
    pub initial_charge_energy: f32,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            light_energy: 10.0,
            light_coef: 0.0008,
            initial_organic_energy: 200.0,
            initial_charge_energy: 200.0,
        }
    }
}

/// Configuration parameters for cell defaults.
#[derive(Reflect, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellDefaults {
    /// Default cell lifespan in ticks
    pub max_ticks_without_energy: u32,
}

impl Default for CellDefaults {
    fn default() -> Self {
        Self {
            max_ticks_without_energy: 3,
        }
    }
}

/// Configuration parameters for the plant simulation, including energy
/// extraction rates, thresholds, and costs.
#[derive(Component, Reflect, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Simulation parameters
    pub simulation: SimulationParameters,
    /// Default parameters for cells
    pub cell_defaults: CellDefaults,
    /// Energy extraction rates for different cell types
    pub extraction_rates: ExtractionRateConfig,
    /// Toxicity thresholds for organic and charge energy
    pub toxicity_thresholds: ToxicityThresholdConfig,
    /// Energy required for a cell to stay alive per tick
    pub cell_living_costs: CellLivingCostConfig,
    /// Energy redistribution parameters for cell death
    pub cell_death_energy_redistribution: CellDeathEnergyRedistributionConfig,
    /// Energy cost for cell actions like moving and reproducing
    pub cell_action_costs: CellActionCostConfig,
    /// Environment configuration parameters
    pub environment: EnvironmentConfig,
}
