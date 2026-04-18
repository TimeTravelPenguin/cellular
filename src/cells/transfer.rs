use std::ops::AddAssign;

use bevy::{
    ecs::{
        entity::Entity,
        message::{MessageReader, MessageWriter},
    },
    platform::collections::HashMap,
    prelude::{Query, With},
};

use crate::{
    cells::{CellEnergyTransferMessage, CellRelation},
    energy::CellEnergy,
};

pub(super) fn cell_pass_energy_system(
    mut query: Query<(Entity, &mut CellEnergy, &CellRelation), With<CellEnergy>>,
    mut transfer_writer: MessageWriter<CellEnergyTransferMessage>,
) {
    for (entity, energy, relation) in query.iter_mut() {
        if energy.0 <= 0.0 {
            continue; // No energy to transfer
        }

        let transfer_amount = energy.0 / relation.children.len() as f32;
        for &child in &relation.children {
            transfer_writer.write(CellEnergyTransferMessage {
                from: entity,
                to: child,
                amount: transfer_amount,
            });
        }
    }
}

pub(super) fn cell_receive_energy_system(
    mut query: Query<(Entity, &mut CellEnergy)>,
    mut transfer_reader: MessageReader<CellEnergyTransferMessage>,
) {
    let mut energy_map = HashMap::new();

    for transfer in transfer_reader.read() {
        energy_map
            .entry(transfer.to)
            .or_insert(0.0)
            .add_assign(transfer.amount);

        // Remove energy from sender
        let (_, mut sender_energy) = query.get_mut(transfer.from).unwrap();
        sender_energy.0 = (sender_energy.0 - transfer.amount).max(0.0);
    }

    // Apply received energy to recipients
    for (entity, mut energy) in query.iter_mut() {
        if let Some(amount) = energy_map.get(&entity) {
            energy.0 += *amount;
        }
    }
}
