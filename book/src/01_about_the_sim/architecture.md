# Architecture

Before getting into the exact outline of the simulation, I want to describe how using the
Bevy ECS helps to structure the simulation nicely.

## Use of Bevy

The Bevy ECS introduces both nice solutions as well as new complications. Unlike the
original simulation, a Rust-based simulation cannot rely on mutable global state;
especially when using Bevy for graphics rendering. By using _systems_ and _components_,
many aspects of the simulation can be decomposed and parallelised.

To give an example, suppose that you have a simulation with cells that, at each step, will
either:

- consume 1 energy to move to a random adjacent tile, if there is empty space
- collect energy from the surrounding 3x3 tile environment

At the very least, you might expect something similar to the following:

```rs
pub struct CellGrid(pub HashMap<(u32, u32), Cell>);

pub struct EnvironmentEnergy(pub HashMap<(u32, u32), u32>);

pub struct Cell {
    pub position: (u32, u32),
    pub energy: u32,
}
```

In languages such as C++, such approaches might work well, assuming you can reason well
about the simulation state. However, even in such languages, parallelising the cell
actions might be difficult or impossible with this approach.

Considering Rust, it won't be easy (or modular) to implement concurrently running steps if
both actions require mutable access to the `Cell` type. There _are_ ways to do so, but it
make things very complex.

Using Bevy, we instead decouple things into _components_. We might, for example, do
something like:

```rs
#[derive(Component, Clone, Debug)]
pub struct Position {
    x: u32,
    y: u32
}

#[derive(Component, Clone, Debug)]
pub struct Energy(u32);

#[derive(Component, Clone, Debug)]
pub struct Cell;

#[derive(Component, Clone, Debug)]
pub struct Moving;
```

Using Bevy, we can now write _systems_. For example, if we have _entities_ that has a
`Cell`, `Position`, and `Energy` _component_, we could make a system like:

```rs
pub fn move_cells_system(
  mut positions: Query<&mut Position, With<(Cell, Moving)>>,
) {
    for (mut position) in positions.iter_mut() {
        // update the position
    }
}

pub fn cell_gather_energy_system(
  mut cell_energies: Query<&mut Energy, (With<Cell>, Without<Moving>)>,
) {
    for (mut energy) in cell_energies.iter_mut() {
        energy.0 += 1;
    }
}
```

With this setup, both systems are able to mutably access their required components in
parallel. Furthermore, Bevy will automatically parallelise wherever possible.
