use bevy::{
    ecs::{
        entity::Entity,
        message::{MessageReader, MessageWriter},
        system::Res,
    },
    prelude::{Commands, Query, ResMut, info},
};

use crate::{
    CellPositions, GridPosition, SimulationSettings,
    cells::{
        Cell, CellRelation, RemainingTicksWithoutEnergy, RemoveChildCellMessage,
        RequestDeathMessage,
    },
    energy::{CellEnergy, ChargeEnergyEnvironment, OrganicEnergyEnvironment, disperse_on_death},
};

pub(super) fn apply_living_cost_system(
    mut query: Query<(
        Entity,
        &mut CellEnergy,
        &Cell,
        &mut RemainingTicksWithoutEnergy,
    )>,
    settings: Res<SimulationSettings>,
    mut writer: MessageWriter<RequestDeathMessage>,
) {
    info!("Applying living costs for {} cells", query.iter().count());

    for (entity, mut energy, cell, mut remaining) in query.iter_mut() {
        let cost = match *cell {
            Cell::Leaf => settings.config.cell_living_costs.leaf_living_cost,
            Cell::Root => settings.config.cell_living_costs.root_living_cost,
            Cell::Antenna => settings.config.cell_living_costs.antenna_living_cost,
            Cell::Branch => settings.config.cell_living_costs.branch_living_cost,
            Cell::Sprout => settings.config.cell_living_costs.sprout_living_cost,
            Cell::Seed => settings.config.cell_living_costs.seed_living_cost,
        };

        **energy = (**energy - cost).max(0.0);

        if **energy <= 0.0 {
            if **remaining == 0 {
                writer.write(RequestDeathMessage { entity });
            } else {
                **remaining -= 1;
            }
        }
    }
}

pub(super) fn handle_cell_death_system(
    mut commands: Commands,
    query: Query<(&Cell, &GridPosition, &CellRelation, &CellEnergy)>,
    mut organic_env: ResMut<OrganicEnergyEnvironment>,
    mut charge_env: ResMut<ChargeEnergyEnvironment>,
    settings: Res<SimulationSettings>,
    mut cell_positions: ResMut<CellPositions>,
    mut reader: MessageReader<RequestDeathMessage>,
    mut writer: MessageWriter<RemoveChildCellMessage>,
) {
    for msg in reader.read() {
        let Ok((cell, pos, relation, energy)) = query.get(msg.entity) else {
            continue;
        };

        relation.children.iter().for_each(|&child| {
            writer.write(RemoveChildCellMessage {
                parent: msg.entity,
                child,
            });
        });

        if let Some(parent) = relation.parent {
            writer.write(RemoveChildCellMessage {
                parent,
                child: msg.entity,
            });
        }

        disperse_on_death(
            *cell,
            *pos,
            **energy,
            &mut organic_env,
            &mut charge_env,
            &settings.config.cell_death_energy_redistribution,
        );

        cell_positions.remove(pos);
        commands.entity(msg.entity).despawn();
    }
}

/// Processes `RemoveChildCellMessage`s emitted by the death handler, pruning
/// dead entities from the surviving parent's or child's `CellRelation`.
pub(super) fn apply_remove_child_system(
    mut relations: Query<&mut CellRelation>,
    mut reader: MessageReader<RemoveChildCellMessage>,
) {
    for msg in reader.read() {
        if let Ok(mut parent_rel) = relations.get_mut(msg.parent) {
            parent_rel.children.remove(&msg.child);
        }
        if let Ok(mut child_rel) = relations.get_mut(msg.child)
            && child_rel.parent == Some(msg.parent)
        {
            child_rel.parent = None;
        }
    }
}
