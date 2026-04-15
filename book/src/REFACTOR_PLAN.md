# Refactor Plan: Request/Resolve Architecture for Cellular Simulation

> [!NOTE]
> This file was generated using Claude

## Context

The simulation is a Rust/Bevy port of the CLANS3 Processing/Java artificial life
simulation. The current implementation has core infrastructure (genome execution, energy
environments, rendering) but is missing critical mechanics (growth, death recycling,
energy transport topology, seed movement, mutation) and uses a flat system chain instead
of the parallel request/resolve architecture described in the book documentation.

The goal is to restructure around **immutable request systems** (parallel) followed by
**mutable resolve systems** (sequential), enabling Bevy's automatic parallelism while
maintaining fairness guarantees.

## SIMULATION.md Discrepancies

Before starting work, these errors/discrepancies in `clan/SIMULATION.md` should be noted:

1. **Leaf formula description is slightly misleading**: Says "free_neighbor_count starts
   at 10". The original code uses `LIGHTENERGY=10` as a base multiplier that decrements by
   1 per occupied neighbor. Functionally equivalent but "count" is misleading since it's a
   light efficiency factor, not a literal count of free neighbors.
2. **No other significant errors found** -- the document is accurate regarding genome
   structure (32x21=672 bytes), growth costs (WORK=5 + ORGANIC_CELL=15 = 20), poison
   thresholds (512), and seed mechanics.

## Key Differences: Rust Implementation vs Original

- Rust genome uses **52 `GenomeEntry` structs** (high-level abstraction) vs original's
  **32 raw 21-byte genes**. This is intentional.
- Rust `GenomePrecondition` has **8 variants** vs original's **68 condition types**. Many
  missing.
- Rust toxicity thresholds are **100/90** vs original's **512/512**. May be intentional
  tuning.
- Rust energy environments initialized to **50/20** vs original's **200/200**. May be
  intentional.

## Source Reference Key

All line references are to the original Processing source in `clan/`:

- **Cells.pde** — Cell class, step logic, commands, conditions, energy transport, growth
- **constant.pde** — All simulation constants
- **func.pde** — Helper functions, dispersal, simulation step, initialization
- **CLANS3eng.pde** — Main loop, setup, rendering

---

## Work Items

### Phase 0: Core Infrastructure

Everything else depends on these foundational pieces.

- [ ] **0.1 — `EnergyEnvironment::deposit()` method** (`src/energy/mod.rs`)

  Add `deposit(x, y, amount)` to write energy back into the grid. Currently only `collect`
  and `peek` exist. Needed for: death recycling, organic scatter, energy dump from isolated
  cells, soil manipulation commands.

  The original writes directly to the global arrays (`OrganicMap[x][y] += value`,
  `EnergyMap[x][y] += value`) in many places:
  - `transmitEnergy()` dumps to soil: `Cells.pde:1028`
  - `OrganicAround()` scatters organic: `Cells.pde:1216-1224`
  - `moveZarad*()` moves charge between tiles: `Cells.pde:1133-1168`
  - `moveOrganic*()` moves organic between tiles: `Cells.pde:1172-1208`

- [ ] **0.2 — `EnergyEnvironment::distribute_around()` method** (`src/energy/mod.rs`)

  Add 9-cell averaging distribution matching the original's dispersal logic. Two functions
  in the original:

  **`distributeOrganic(x, y, E)`** (`func.pde:143-158`):
  - Sums the existing values in the 3x3 neighborhood plus `E`
  - Integer divides by 9 (base share `b`), remainder `c` stays at center
  - Each of the 9 cells gets `b`, center gets `b + c`
  - Note: this **replaces** existing values, it does not add to them

  **`distributeZarad(x, y, E)`** (`func.pde:163-178`):
  - Same logic but for energy (float in original)
  - Sums 3x3 + E, divides by 9, distributes evenly
  - All 9 cells set to `b` (remainder handling differs slightly from organic)

  Both are called during `die()` (`Cells.pde:1038-1039`).

- [ ] **0.3 — Live `SimulationGrid` spatial index** (`src/simulation.rs`, `src/main.rs`)

  `SimulationGrid` exists but is never inserted as a resource or maintained. Change
  `cells` field to `HashMap<(usize, usize), Entity>`. Insert as resource on startup.
  Add/remove/update entries on spawn, death, and movement.

  The original uses `cellsIndx[X][Y]` (`Cells.pde:288, 1072, 1104, etc.`) as a global 2D
  array mapping grid positions to cell indices. This is checked:
  - During growth to verify target is empty: `Cells.pde:288`
  - During movement to check collisions: `Cells.pde:1072, 1104`
  - During seed collision: `Cells.pde:347`
  - In `calculateSunEnergy()` for neighbor detection: `Cells.pde:1505-1536`
  - In `findIndexFromRelDirection()`: `Cells.pde:1468-1482`
  - In `isFreeInRelDirection()`: `Cells.pde:1422-1436`
  - In condition checks for obstacles/edible cells: `Cells.pde:500-562`
  - Updated on death: `Cells.pde:1053` (`cellsIndx[X][Y] = 0`)
  - Updated on movement: `Cells.pde:1107-1109`

- [ ] **0.4 — `CellAge` component** (`src/cells/mod.rs`)

  `u32`, default = `AGE` (3) (`constant.pde:8`). Decrements when cell energy reaches zero.
  Cell dies when age reaches 0.

  Age decrement happens in multiple cell types:
  - WOOD: `Cells.pde:103` (`age--` when `energy < 0`)
  - LEAF: `Cells.pde:115` (same pattern)
  - ROOT: `Cells.pde:135` (same pattern)
  - ANTN: `Cells.pde:155` (same pattern)
  - Death check: `Cells.pde:106, 120, 140, 160` (`if(age <= 0) die()`)

  Also decremented when energy can't be transmitted: `Cells.pde:1029` (no parent, dump to
  soil).

- [ ] **0.5 — `CellLevel` component** (`src/cells/mod.rs`)

  `u32`. Tracks growth depth from organism root.

  Set during cell creation: `Cells.pde:298` (`level = level + 1`).
  Reset when alone: `Cells.pde:167` (`if(parent == -1) level = 0`).
  Also reset when seed hatches: `Cells.pde:91` (`level = 0`).
  Used in conditions 4-7 (`Cells.pde:439-457`):
  - Cond 4: `param % (level+1) == 0`
  - Cond 5: `level % (param+1) == 0`
  - Cond 6: `level > param`
  - Cond 7: `level < param`

- [ ] **0.6 — `CellOrganic` component** (`src/cells/mod.rs`)

  `u32`, default = `ORGANIC_CELL` (15) (`constant.pde:17`). Released to soil on death via
  `distributeOrganic()` (`Cells.pde:1038`).

  Set on cell creation: `Cells.pde:296` (`org = ORGANIC_CELL`).
  Consumed by `ConsumeNeighbours` command: `Cells.pde:974` (absorbs `org` from killed
  neighbors).

- [ ] **0.7 — `PreviousEnergy` component** (`src/energy/mod.rs`)

  Track previous tick's energy for conditions 8/9. The original stores `energyOld`
  implicitly — conditions 8/9 compare current `energy` to `energyOld`:
  - Condition 8 (`Cells.pde:460`): `energy >= energyOld` → rising
  - Condition 9 (`Cells.pde:464`): `energy < energyOld` → falling

  The original doesn't explicitly copy `energyOld = energy` at tick start — `energyOld` is
  set when energy changes during transmission. For simplicity in Bevy, copy `CellEnergy`
  into `PreviousEnergy` at the start of each tick.

- [ ] **0.8 — Activate `CellRelation`** (`src/cells/mod.rs`)

  Component exists but is never inserted. Every spawned cell must have correct
  parent/children. The original uses `parent` (direction to parent, -1 if none) and
  `children[4]` (flags for each cardinal direction).

  Set during growth: `Cells.pde:300-301` (child gets `parent = invert(absDir)`, parent
  gets `children[absDir] = 1`).
  Broken by `destroyAllLinks()` (`Cells.pde:1543-1557`):
  - Iterates all 4 directions
  - If `children[i] == 1`: set child's `parent = -1`
  - Clears parent's `energyTo` toward this cell
  - Sets own `parent = -1`

  In Bevy, store parent as `Option<Entity>` and children as `Vec<Entity>` (already the
  case in the existing `CellRelation` struct).

- [ ] **0.9 — Wire `EnergyTransferer` to `CellRelation` topology** (`src/energy/mod.rs`)

  Branch cells set `energyTo` based on children. LEAF/ROOT/ANTN point toward parent.

  Set during growth (`Cells.pde:327-333`):
  - Parent (now WOOD): `energyTo[absDir] = 1` if child is APEX
  - Child (LEAF/ROOT/ANTN): `energyTo[invert(absDir)] = 1` (toward parent)

  When transmission fails (`Cells.pde:1021-1025`):
  - Cell sets `energyTo[parent] = 1` (start sending to parent)
  - Tells parent to stop sending back: parent's `energyTo[invert(parent)] = 0`

- [ ] **0.10 — `EnergyTransportPhase` resource** (`src/energy/mod.rs`)

  Flips between `+1` and `-1` each tick. Toggled in `simulationStep()`
  (`func.pde:69`): `EnergyTransportPeriod *= -1`.

  At the start of each cell's `step()` (`Cells.pde:64-65`):
  - If phase is `+1`: `energy += engM; engM = 0`
  - If phase is `-1`: `energy += engP; engP = 0`

  During `transmitEnergy()` (`Cells.pde:1005-1018`):
  - If phase is `+1`: writes to recipient's `engP`
  - If phase is `-1`: writes to recipient's `engM`

  This ensures energy propagates at most 1 cell per tick.

- [ ] **0.11 — Fix toroidal grid wrapping in `GridPosition::offset()`**
      (`src/main.rs:58-63`)

  Currently uses `.max(0)` which clamps instead of wrapping.
  `GridBoundary::Wrap` exists in `simulation.rs` but is unused.

  The original wraps in `X()` and `Y()` (`func.pde:193-203`):

  ```
  if(x >= W) x = x - W;
  else if(x < 0) x = W + x;
  ```

  `GridPosition::offset()` needs access to grid dimensions. Options:
  - Take `SimulationSettings` or `(width, height)` as parameter
  - Store dimensions in a global or make `offset` a method on `SimulationGrid`

---

### Phase 1: Request/Resolve Architecture

Restructure the system execution pipeline per the book's PlantUML diagram
(`book/src/02_details/systems.md`).

- [ ] **1.1 — Define Bevy `SystemSet`s** (`src/main.rs`)
  - `GenomeActionSet` (parallel) — genome execution produces request components
  - `EnergyProducerSet` (parallel) — Root/Antenna/Leaf energy collection + transfer
    requests
  - `ResolveRequestSet` (sequential) — process move, take, spawn, death requests
  - `BranchTransferSet` — Branch cells produce deposit requests
  - `ResolveDepositSet` (sequential) — process deposit transfers
  - `MaintenanceSet` (sequential) — energy costs, age, death checks, cleanup

- [ ] **1.2 — Define request component types** (`src/cells/mod.rs` or new module)
  - `RequestMove { target_position: GridPosition }` — seed/apex movement
  - `RequestSpawnCell { direction: RelativeDirection, cell_type: Cell, genome: Genome,
active_gene: GenomeID, level: u32 }` — growth
  - `RequestTakeEnergy { source_position: GridPosition, energy_type: Energy, amount: u32
}` — pulling from soil
  - `RequestDepositEnergy { to_entity: Entity, amount: u32 }` — energy transfer via
    topology
  - `RequestDeath` — scheduled death (replaces current ad-hoc `CellIsDying`)
  - `RequestDetach` — break parent-child links
  - `RequestMoveEnvironment { from_pos: GridPosition, to_pos: GridPosition, energy_type:
Energy }` — soil manipulation

  These replace the existing marker-based approach (`CellRequestSolarEnergy`, etc.).

- [ ] **1.3 — Make `invoke_cell_genome_actions_system` read-only**
      (`src/cells/systems.rs`)

  Instead of directly mutating `Cell` and `GenomeID`, attach request components. The
  genome execution (`genome.execute()`) is already pure. The match arms should emit
  requests instead of `todo!()` or direct mutation.

  Key change: the system needs `Commands` access to insert request components on entities
  but should NOT mutate `Cell` or `GenomeID` directly. Those mutations happen in resolve
  systems.

  The existing `cell_positions: HashSet<GridPosition>` collection for obstacle detection
  should be replaced with `SimulationGrid` reads (Phase 0.3).

- [ ] **1.4 — `resolve_move_requests_system`** (`src/cells/systems.rs`)

  Process `RequestMove`. Check `SimulationGrid` for collisions.

  Original movement logic:
  - **APEX movement** (`Cells.pde:1093-1113`): Deduct `moveApexPrice` (1.0) energy. Check
    target cell empty. If occupied → fail. If free → update `cellsIndx` at old and new
    positions, update X/Y.
  - **Seed movement** (`Cells.pde:1061-1089`): Deduct 1 energy. Check target. If occupied
    → **kill the occupying cell** and stop (seed stays, `restTime = 0`). If free → move.

  The resolve system should:
  1. Query all entities with `RequestMove`
  2. For each, check `SimulationGrid` at target position
  3. Handle collision per cell type (seed kills target; apex fails)
  4. Update `GridPosition`, `Transform`, and `SimulationGrid`
  5. Remove `RequestMove` component
  6. Track success/failure for genome next-gene branching

- [ ] **1.5 — `resolve_spawn_requests_system`** (`src/cells/systems.rs`)

  Process `RequestSpawnCell`. Original growth logic (`Cells.pde:274-334`):
  1. Parent APEX becomes WOOD: `Cells.pde:275` (`type = WOOD`)
  2. Calculate absolute direction from relative: `Cells.pde:279-286`
  3. Check target cell is empty: `Cells.pde:288`
  4. Create new cell with:
     - `age = AGE` (3): `Cells.pde:295`
     - `org = ORGANIC_CELL` (15): `Cells.pde:296`
     - `type` from gene mapping: `Cells.pde:297`
     - `level = parent.level + 1`: `Cells.pde:298`
     - `direction = absDir`: `Cells.pde:300`
     - `parent = invert(absDir)`: `Cells.pde:301`
     - `adam = parent.adam`: `Cells.pde:303`
     - `gn = parent.gn`: `Cells.pde:311`
  5. Set parent's `children[absDir] = 1`: `Cells.pde:327`
  6. Set parent's `energyTo[absDir] = 1` if child is APEX: `Cells.pde:329`
  7. Set child's `energyTo[invert(absDir)] = 1` if LEAF/ROOT/ANTN: `Cells.pde:331-332`
  8. Mutation check (1% for APEX children): `Cells.pde:314-323`
  9. Deduct energy from parent: `needEnergy = count * (WORK + ORGANIC_CELL)` per
     `Cells.pde:229-230`

- [ ] **1.6 — `resolve_death_requests_system`** (`src/cells/systems.rs`)

  Process `RequestDeath` + `CellIsDying`. Original death sequence (`Cells.pde:1036-1058`):
  1. `transmitEnergy()` — final energy transfer to parent/soil: `Cells.pde:1037`
  2. `distributeOrganic(X, Y, org)` — spread 15 organic to 3x3: `Cells.pde:1038`
  3. `distributeZarad(X, Y, energy+engP+engM)` — spread energy to 3x3: `Cells.pde:1039`
  4. `destroyAllLinks()` — break all parent/child connections
  5. Clear `cellsIndx[X][Y]`: `Cells.pde:1053`
  6. Remove from linked list: `Cells.pde:1054-1056`
  7. Return to free pool: `Cells.pde:1042-1043`

  In Bevy: despawn entity, update `SimulationGrid`, distribute resources via
  `distribute_around`, break `CellRelation` links on parent/children entities.

- [ ] **1.7 — `resolve_detach_requests_system`** (`src/cells/systems.rs`)

  Process `RequestDetach`. Original `destroyAllLinks()` (`Cells.pde:1543-1557`):
  1. For each direction (0-3):
     - If `children[i] == 1`: set `cells[childIndex].parent = -1`
     - Set neighbor's `energyTo[invert(i)] = 0` (stop them sending energy to us)
  2. Clear all own `energyTo[]` flags
  3. Set `parent = -1`

  In Bevy: remove this entity from parent's `CellRelation.children`, clear parent's
  `EnergyTransferer` entry for this entity, set own `CellRelation.parent = None`, clear
  own `EnergyTransferer`.

- [ ] **1.8 — `resolve_environment_move_system`** (`src/energy/systems.rs`)

  Process `RequestMoveEnvironment` for soil manipulation commands 6-11.

  Original implementations:
  - `moveZaradLeft/Ahead/Right()` (`Cells.pde:1133-1168`): Move ALL energy from cell's
    position to the target position. `EnergyMap[target] += EnergyMap[X][Y];
EnergyMap[X][Y] = 0`
  - `moveOrganicLeft/Ahead/Right()` (`Cells.pde:1172-1208`): Same for organic.

  These are "push" operations — they move the resource from the cell's own tile to an
  adjacent tile.

- [ ] **1.9 — Rewire `main.rs` system registration** (`src/main.rs:180-198`)

  Replace flat `.chain()` with system sets and proper ordering constraints.

---

### Phase 2: Energy System Completion

- [ ] **2.1 — Fix solar energy formula** (`src/energy/systems.rs`)

  Current implementation (`src/energy/systems.rs:72-81`) just adds `sunlight` value to
  cell energy. The correct formula from `calculateSunEnergy()` (`Cells.pde:1502-1538`):

  ```
  mn = LIGHTENERGY  // 10
  for each of 8 neighbors:
      if neighbor is LEAF → return 0  // complete shading
      if neighbor exists (any cell) → mn -= 1
  return OrganicMap[X][Y] * mn * LIGHTCOEF  // organic * (10 - obstructions) * 0.0008
  ```

  Key details:
  - Checks all 8 cardinal + diagonal neighbors: `Cells.pde:1505-1536`
  - If ANY neighbor is a LEAF → energy is **zero** (mutual shading rule)
  - Non-leaf occupied neighbors reduce `mn` by 1 each
  - `mn` can go to 0 if all 8 neighbors are occupied (but not LEAF)
  - Requires `SimulationGrid` for neighbor cell type lookup
  - The `SunlightCycle` resource is NOT used in the original — sunlight is purely derived
    from soil organic content. The `SunlightCycle` appears to be a custom addition.

- [ ] **2.2 — Energy cost system** (`src/energy/systems.rs`)

  Per-tick costs from `constant.pde:9-11`:
  - `ApexEnergy4Life = 1.0` (Sprout): `Cells.pde:171`
  - `SeedEnergy4Life = 0.5` (Seed): `Cells.pde:74`
  - `Energy4Life = 0.04` (WOOD, LEAF, ROOT, ANTN): `Cells.pde:102, 113, 134, 154`

  When energy goes negative:
  - `age--; energy = 0`: `Cells.pde:103, 115, 135, 155`
  - APEX/SEED check `energy + engP + engM < 0` instead (includes buffers):
    `Cells.pde:73, 166`

  Death trigger:
  - `if(age <= 0) die()`: `Cells.pde:106, 120, 140, 160`
  - LEAF/ROOT/ANTN also die if `parent == -1`: `Cells.pde:120, 140, 160`

  Note: since Rust uses `u32` for energy (not float), fractional costs (0.04, 0.5) need
  either: (a) switch to `f32`, (b) accumulate a fractional counter, or (c) scale all
  energy values by 100 and use integer math.

- [ ] **2.3 — Ping-pong energy transfer for Branch cells** (`src/energy/systems.rs`)

  Original `transmitEnergy()` (`Cells.pde:1001-1031`):

  ```
  n = count of energyTo[] flags set to 1
  if n > 0:
      en = energy / n
      if EnergyTransportPeriod == 1:
          for each target with energyTo[i] == 1:
              cells[target].engP += en
          energy = 0
      else:
          for each target with energyTo[i] == 1:
              cells[target].engM += en
          energy = 0
  ```

  At step start (`Cells.pde:64-65`):

  ```
  if EnergyTransportPeriod == 1:
      energy += engM; engM = 0
  else:
      energy += engP; engP = 0
  ```

  In Bevy, add two buffer components (e.g., `EnergyBufferP(f32)`, `EnergyBufferM(f32)`)
  or a single `EnergyTransferBuffer { p: f32, m: f32 }`. The `EnergyTransportPhase`
  resource determines which buffer to write to and which to read from.

  Called by: LEAF (`Cells.pde:116`), ROOT (`Cells.pde:136`), ANTN (`Cells.pde:156`), WOOD
  (`Cells.pde:104`). All energy-producing and transport cells call this when they have
  positive energy.

- [ ] **2.4 — "No recipients" energy fallback** (`src/energy/systems.rs`)

  When `n == 0` (no `energyTo` targets) in `transmitEnergy()` (`Cells.pde:1021-1030`):

  ```
  if parent != -1:
      // Start sending to parent next time
      energyTo[parent] = 1
      // Tell parent to stop sending energy back to us
      cells[parentIndex].energyTo[invert(parent)] = 0
  else:
      // Dump to soil
      EnergyMap[X][Y] += energy
      age--
      energy = 0
  ```

  This is the self-correcting mechanism: cells that can't transmit redirect toward their
  parent, and isolated cells lose energy to the soil with an age penalty.

- [ ] **2.5 — `PreviousEnergy` tracking system** (`src/energy/systems.rs`)

  At tick start, copy `CellEnergy` to `PreviousEnergy`. Used by conditions 8/9
  (`Cells.pde:459-467`). Simple system that runs before all others in the tick.

---

### Phase 3: Genome Execution Completion

- [ ] **3.1 — Implement 6 remaining multi-cell commands** (`src/cells/systems.rs`)

  All from `command()` (`Cells.pde:795-867`):

  | Cmd  | Rust Enum                              | Original Logic                                                             | Source              |
  | ---- | -------------------------------------- | -------------------------------------------------------------------------- | ------------------- |
  | 0    | `SkipTurn`                             | No-op                                                                      | `Cells.pde:799`     |
  | 1    | `BecomeASeed` (flying)                 | `type=SEED, destroyAllLinks(), move=true, restTime=8`                      | `Cells.pde:802-808` |
  | 2    | `BecomeASeed` (stationary)             | `type=SEED, move=false, restTime=8`                                        | `Cells.pde:810-813` |
  | 3    | `BecomeADetachedSeed`                  | `type=SEED, move=true, restTime=8`                                         | `Cells.pde:815-819` |
  | 4    | **`Die`**                              | `energyTo[parent]=1, die()`                                                | `Cells.pde:821-824` |
  | 5    | **`SeparateFromOrganism`**             | `destroyAllLinks()`                                                        | `Cells.pde:826-828` |
  | 6-8  | **`TransportSoilEnergy(dir)`**         | `moveZaradLeft/Right/Ahead()`                                              | `Cells.pde:831-841` |
  | 9-11 | **`TransportSoilOrganicMatter(dir)`**  | `moveOrganicLeft/Right/Ahead()`                                            | `Cells.pde:843-852` |
  | 12   | **`ShootSeed { high_energy: false }`** | `setNewSEED(0, 30)` — bullet: 30 energy, 30 tick flight                    | `Cells.pde:855-857` |
  | 13   | **`ShootSeed { high_energy: true }`**  | `setNewSEED(1, 5+random(40))` — reproductive: all energy, 5-44 tick flight | `Cells.pde:859-861` |
  | 14   | **`DistributeEnergyAsOrganicMatter`**  | `OrganicAround()`                                                          | `Cells.pde:863-865` |

  **`OrganicAround()` detail** (`Cells.pde:1213-1226`):

  ```
  if energy < 12: return (fail)
  ee = floor((energy - 3) / 9)
  for each of 9 cells in 3x3:
      OrganicMap[cell] += ee
  energy = 3
  ```

  **`setNewSEED(en, rt)` detail** (`Cells.pde:339-399`):
  - Energy check: `if(energy < ORGANIC_CELL + WORK + 30) return false` (line 340)
  - Check forward cell (line 347):
    - If occupied by same genome → add energy to it
    - If occupied by different genome → kill it
  - Subtract `ORGANIC_CELL + WORK` from parent (line 353)
  - Create SEED entity with:
    - `type = SEED`, `level = 0`, `parent = -1` (lines 363-370)
    - `direction = parent's direction` (line 372)
    - `restTime = rt` (line 386), `move = true` (line 387)
    - `adam = parent.adam` (line 375), `gn = parent.gn` (line 376)
  - Energy transfer (lines 389-396):
    - If `en == 0` (bullet): seed gets 30 energy, rest stays with parent
    - If `en == 1` (reproductive): seed gets ALL remaining parent energy

  **`moveZarad*` / `moveOrganic*` detail** (`Cells.pde:1133-1208`):
  - Calculates target position (left/ahead/right relative to facing)
  - Moves ALL of that resource from cell's position to target:
    `Map[target] += Map[X][Y]; Map[X][Y] = 0`
  - Always returns `true`

- [ ] **3.2 — Implement 18 single-cell commands** (`src/cells/systems.rs`)

  All from `command_alone()` (`Cells.pde:873-996`):

  | Cmd | Rust Enum                | Original Logic                                              | Source              |
  | --- | ------------------------ | ----------------------------------------------------------- | ------------------- |
  | 0   | `MoveForward`            | `moveApex()`                                                | `Cells.pde:876-878` |
  | 1   | `TurnRight`              | `direction += 1; direction %= 4`                            | `Cells.pde:880-883` |
  | 2   | `TurnLeft`               | `direction -= 1; if < 0 then += 4`                          | `Cells.pde:885-888` |
  | 3   | `TurnAround`             | `direction += 2; direction %= 4`                            | `Cells.pde:890-893` |
  | 4   | `TurnRightAndMove`       | Turn right + `moveApex()`                                   | `Cells.pde:895-899` |
  | 5   | `TurnLeftAndMove`        | Turn left + `moveApex()`                                    | `Cells.pde:901-905` |
  | 6   | `TurnAroundAndMove`      | Turn around + `moveApex()`                                  | `Cells.pde:907-911` |
  | 7   | `Parasitise`             | Attach to forward WOOD cell                                 | `Cells.pde:913-924` |
  | 8   | `TurnRandom`             | `r=random(0,10); if r<3 right, elif r<6 left, else nothing` | `Cells.pde:926-932` |
  | 9   | `MoveRandom`             | Random turn (same as 8) + `moveApex()`                      | `Cells.pde:934-937` |
  | 10  | `PullOrganicFromLeft`    | `pushOrganicFromLeft()`                                     | `Cells.pde:940-942` |
  | 11  | `PullOrganicFromForward` | `pushOrganicFromAhead()`                                    | `Cells.pde:944-946` |
  | 12  | `PullOrganicFromRight`   | `pushOrganicFromRight()`                                    | `Cells.pde:948-950` |
  | 13  | `PullChargeFromLeft`     | `pushZaradFromLeft()`                                       | `Cells.pde:952-954` |
  | 14  | `PullChargeFromForward`  | `pushZaradFromAhead()`                                      | `Cells.pde:956-958` |
  | 15  | `PullChargeFromRight`    | `pushZaradFromRight()`                                      | `Cells.pde:960-962` |
  | 16  | `ConsumeNeighbours`      | Kill adjacent non-WOOD/non-SEED cells                       | `Cells.pde:965-982` |
  | 17  | `TakeEnergyFromSoil`     | Absorb up to 6 from soil                                    | `Cells.pde:985-994` |

  **`moveApex()` detail** (`Cells.pde:1093-1113`):
  - Deduct `moveApexPrice` (1.0): line 1094
  - Calculate forward position: lines 1098-1101
  - If occupied → `return false`: line 1104
  - Update `cellsIndx` (clear old, set new): lines 1107-1109
  - Update X, Y: lines 1110-1111

  **`Parasitise` detail** (`Cells.pde:913-924`):
  - Find cell in forward direction: `findIndexFromDirection(direction)`
  - If that cell exists AND is WOOD:
    - Set own `parent = direction` (face toward host)
    - Set host's `children[invert(direction)] = 1`
    - Set host's `energyTo[invert(direction)] = 1` (host feeds parasite)
    - `return true`
  - Otherwise `return false`

  **`ConsumeNeighbours` detail** (`Cells.pde:965-982`):
  - Cost: `energy -= 1` (line 966)
  - Check 8 directions (0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5 — includes diagonals): line 968
  - For each direction: `findIndexFromDirection(d)` (line 969)
  - If cell exists AND `type < WOOD` (i.e., APEX, LEAF, ANTN, ROOT): line 971
    - Absorb: `energy += target.energy + target.engP + target.engM + target.org` (line 974)
    - Kill target: `target.die()` (line 975)
  - `return true`

  Note: the original checks 8 directions including diagonals (0.5-step increments). The
  `findIndexFromDirection()` function handles these fractional directions
  (`Cells.pde:1487-1497`).

  **`pushOrganicFrom*` detail** (`Cells.pde:1231-1276`):
  - Calculate source position (left/ahead/right of cell)
  - If `OrganicMap[source] <= 0` → `return false`
  - Move ALL organic: `OrganicMap[X][Y] += OrganicMap[source]; OrganicMap[source] = 0`
  - `return true`

  **`pushZaradFrom*` detail** (`Cells.pde:1280-1324`):
  - Same as organic but for EnergyMap

  **`TakeEnergyFromSoil` detail** (`Cells.pde:985-994`):
  - `ALONE_CAN = 6` (`constant.pde:4`)
  - If `EnergyMap[X][Y] > ALONE_CAN`: take 6, `return true`
  - Else: take all remaining, `return false`

- [ ] **3.3 — Command success/failure tracking** (`src/cells/systems.rs`)

  Commands return `bool` (`res`) in the original. This determines genome branching:
  - Body cell, conditions met: success → `aGen = GN[gn][aG+10] % 32`, fail → `aGen =
GN[gn][aG+11] % 32` (`Cells.pde:188-189`)
  - Body cell, conditions not met: success → `GN[gn][aG+13] % 32`, fail → `GN[gn][aG+14]
% 32` (`Cells.pde:208-209`)
  - Lone cell, conditions met: success → `GN[gn][aG+16] % 32`, fail → `GN[gn][aG+17] %
32` (`Cells.pde:192-193`)
  - Lone cell, conditions not met: success → `GN[gn][aG+19] % 32`, fail → `GN[gn][aG+20]
% 32` (`Cells.pde:212-213`)

  In the request/resolve model, the genome system emits both `success_next_genome` and
  `fail_next_genome` (already in `CellGenomeCommand`). The resolve system determines
  success/failure and writes the appropriate `GenomeID`. This requires a mechanism for
  resolve systems to update `GenomeID` — either:
  - A `PendingGenomeUpdate { success_id, fail_id }` component set by genome system,
    resolved after commands execute
  - Or the resolve systems directly write `GenomeID`

- [ ] **3.4 — Growth system** (`src/cells/systems.rs`)

  The "no conditions" branch in APEX gene execution (`Cells.pde:217-231`):

  ```
  // Count expected branches from gene bytes 0-2
  tempCellCount = 0
  for i in 0..3:
      if GN[gn][aG + i] <= 95:
          tempCellCount++

  needEnergy = tempCellCount * (WORK + ORGANIC_CELL)  // count * 20
  if energy >= needEnergy:
      grow()  // calls setNewSegment for each direction
  ```

  In Rust, `GenomeSpawn` already stores the cell types for forward/left/right. The growth
  system should:
  1. Check if `GenomeEntry.conditionals.preconditions` is empty (no conditions trigger)
  2. Count non-empty spawn directions from `GenomeSpawn`
  3. Check energy >= `count * 20`
  4. Emit `RequestSpawnCell` for each direction
  5. Emit a request to convert self from Sprout → Branch

  The gene-to-cell-type mapping (`Cells.pde:247-262`):
  - 0-63 → APEX (active gene = `value % 32`)
  - 64-75 → LEAF
  - 76-85 → ANTN
  - 86-95 → ROOT
  - 96+ → no growth

  This mapping is already abstracted away by `GenomeSpawn` in Rust — the `Cell` enum
  variant is stored directly.

- [ ] **3.5 — APEX/Seed detach-on-excess** (`src/cells/systems.rs`)

  APEX detach (`Cells.pde:173-176`):

  ```
  if energy > MAX_APEX_ENERGY (1024) && parent != -1:
      destroyAllLinks()
      aGen = 0
  ```

  Seed detach (`Cells.pde:76`):

  ```
  if energy > MAX_SEED_ENERGY (512) && parent != -1:
      destroyAllLinks()
  ```

  Constants from `constant.pde:20-21`.

---

### Phase 4: Precondition Expansion

- [ ] **4.1 — Expand `GenomePrecondition`** (`src/genes.rs`)

  Currently 8 variants, original has 68 (`Cells.pde:409-789`). Full mapping:

  | ID    | Condition                              | Params                         | Source              |
  | ----- | -------------------------------------- | ------------------------------ | ------------------- |
  | 0     | `OrganicAtPosition < param*2`          | organic at (X,Y), param        | `Cells.pde:419-423` |
  | 1     | `OrganicAtPosition >= param*2`         | same                           | `Cells.pde:424-427` |
  | 2     | `CellEnergy > param*2`                 | cell energy, param             | `Cells.pde:429-433` |
  | 3     | `CellEnergy < param*2`                 | same                           | `Cells.pde:434-437` |
  | 4     | `param % (level+1) == 0`               | param, level                   | `Cells.pde:439-443` |
  | 5     | `level % (param+1) == 0`               | level, param                   | `Cells.pde:444-447` |
  | 6     | `level > param`                        | level, param                   | `Cells.pde:449-453` |
  | 7     | `level < param`                        | level, param                   | `Cells.pde:454-457` |
  | 8     | `energy >= energyOld` (rising)         | energy, prev energy            | `Cells.pde:459-463` |
  | 9     | `energy < energyOld` (falling)         | same                           | `Cells.pde:464-467` |
  | 10    | `organicCount9(X,Y) > param*18`        | 3x3 organic sum, param         | `Cells.pde:469-473` |
  | 11    | `organicCount9(X,Y) < param*18`        | same                           | `Cells.pde:474-477` |
  | 12    | `zaradCount9(X,Y) > param*18`          | 3x3 energy sum, param          | `Cells.pde:479-483` |
  | 13    | `zaradCount9(X,Y) < param*18`          | same                           | `Cells.pde:484-487` |
  | 14    | `zaradCount9 > organicCount9`          | both 3x3 sums                  | `Cells.pde:489-493` |
  | 15    | `zaradCount9 < organicCount9`          | same                           | `Cells.pde:494-497` |
  | 16    | Edible cells nearby (5 dirs)           | grid lookup                    | `Cells.pde:500-518` |
  | 17    | Area is free (left+center+right)       | grid lookup                    | `Cells.pde:520-532` |
  | 18-20 | Free space left/center/right           | grid + poison check            | `Cells.pde:533-547` |
  | 21-23 | Obstacle left/center/right             | inverse of 18-20               | `Cells.pde:549-562` |
  | 24    | Has parent                             | parent != -1                   | `Cells.pde:564-567` |
  | 25    | Random                                 | random(256) > param            | `Cells.pde:569-572` |
  | 26-31 | Light comparisons (3 dirs)             | `findLight3FromRelDirection`   | `Cells.pde:575-603` |
  | 32-37 | Energy 9-cell comparisons (3 dirs)     | `findZarad9FromRelDirection`   | `Cells.pde:605-635` |
  | 38-40 | Energy 9-cell thresholds               | `findZarad9FromRelDirection`   | `Cells.pde:637-650` |
  | 41-46 | Organic 9-cell comparisons (3 dirs)    | `findOrganic9FromRelDirection` | `Cells.pde:652-682` |
  | 47-49 | Organic 9-cell thresholds              | `findOrganic9FromRelDirection` | `Cells.pde:684-696` |
  | 50-55 | Free space 9-cell comparisons (3 dirs) | `howManySpace9InRelDirection`  | `Cells.pde:698-728` |
  | 56-58 | Free space 9-cell thresholds           | `howManySpace9InRelDirection`  | `Cells.pde:730-742` |
  | 59-61 | Organic poison ahead/left/right        | `yadFromRelDirection(0)`       | `Cells.pde:744-762` |
  | 62-64 | Energy poison ahead/left/right         | `yadFromRelDirection(1)`       | `Cells.pde:764-775` |
  | 65-67 | Any poison ahead/left/right            | `yadFromRelDirection(2)`       | `Cells.pde:777-787` |

  Note: the Rust genome uses a different encoding (high-level enum vs raw bytes), so not
  all 68 need 1:1 mapping. But the categories that matter most for evolution are:
  - **Spatial awareness**: 16-24, 50-58 (free space, obstacles, edible cells)
  - **Resource sensing**: 10-15, 26-49 (directional organic/energy gradients)
  - **Poison avoidance**: 59-67
  - **Organism state**: 2-9, 24 (energy, level, parent)

- [ ] **4.2 — Expand `PreconditionParameters`** (`src/genes.rs`)

  Currently:

  ```rust
  pub struct PreconditionParameters {
      pub organic_energy: NeighbouringEnergy,
      pub charge_energy: NeighbouringEnergy,
      pub cell_energy_has_increased: bool,
      pub obstacles: ObstacleInfo,
      pub rng_value: u8,
  }
  ```

  Needs additions:
  - `has_parent: bool`
  - `level: u32`
  - `cell_energy: u32` (for conditions 2-3)
  - `previous_energy: u32` (for conditions 8-9, replaces `cell_energy_has_increased`)
  - `organic_at_position: u32` (for conditions 0-1)
  - `organic_9cell: u32` (for conditions 10-11)
  - `charge_9cell: u32` (for conditions 12-13)
  - Directional 9-cell data (for conditions 26-58): `organic_9_forward`, `organic_9_left`,
    `organic_9_right`, and same for charge and free-space
  - Directional light data (for conditions 26-31): `light3_forward`, `light3_left`,
    `light3_right`
  - Poison data (for conditions 59-67): `poison_forward`, `poison_left`, `poison_right`
    (each as flags for organic/energy/any)
  - `edible_cells_nearby: bool` (for condition 16)
  - `area_is_free: bool` (for condition 17)
  - Free space in each direction (for conditions 18-20)

- [ ] **4.3 — Directional 9-cell scans** (`src/energy/mod.rs`)

  The original's directional scans are NOT the same as the current `NeighbouringEnergy`
  3x3:

  **`findOrganic9FromRelDirection(relDir)`** (`Cells.pde:1383-1398`):
  - Converts relative direction to absolute
  - Calculates a 3x3 block that is **offset 1-3 cells ahead** in that direction
  - Sums OrganicMap values in that 3x3 block

  **`findZarad9FromRelDirection(relDir)`** (`Cells.pde:1403-1417`):
  - Same but for EnergyMap

  **`howManySpace9InRelDirection(relDir)`** (`Cells.pde:1441-1463`):
  - Counts free cells in the offset 3x3 block
  - "Free" means: `cellsIndx == 0 AND OrganicMap < EXCESS AND EnergyMap < EXCESS`
    (`isFreeInRelDirection`, `Cells.pde:1422-1436`)

  **`findLight3FromRelDirection(relDir)`** (`Cells.pde:1330-1348`):
  - Sums OrganicMap for 3 cells ahead in the relative direction
  - Excludes cells where OrganicMap >= ORGANIC_EXCESS (poison)

  These helper functions compute data used by conditions 26-58.

---

### Phase 5: Seed Mechanics

- [ ] **5.1 — `SeedRestTime` component** (`src/cells/mod.rs`)

  `u32`. Set during seed creation:
  - Bullet (command 12): `restTime = 30` (`Cells.pde:855` → `setNewSEED(0, 30)`)
  - Reproductive (command 13): `restTime = 5 + random(40)` (`Cells.pde:859` →
    `setNewSEED(1, 5+random(40))`)
  - BecomeASeed commands: `restTime = 8` (`Cells.pde:806, 812, 818`)

  At 0, seed transforms into Sprout at gene 0: `Cells.pde:89-91`
  (`age=AGE, aGen=0, type=APEX, level=0`).

- [ ] **5.2 — Seed flight** (`src/cells/mod.rs`)

  `moveSeed()` (`Cells.pde:1061-1089`):
  - Deduct 1 energy: line 1062
  - Calculate forward position: lines 1066-1069
  - If occupied: **kill the occupying cell** (`cells[inx].die()`), stop moving
    (`restTime = 0, move = false`): lines 1073-1076
  - If wall (negative index): stop moving: lines 1078-1080
  - If free: move to position, update `cellsIndx`: lines 1083-1087

  Only moves if `parent == -1 AND move == true AND restTime > 0`: `Cells.pde:81-82`.

- [ ] **5.3 — `seed_behavior_system`** (`src/cells/systems.rs`)

  Full seed step logic (`Cells.pde:72-95`):

  ```
  if(energy + engP + engM < 0) die()
  energy -= SeedEnergy4Life  // 0.5
  if(energy > MAX_SEED_ENERGY && parent != -1) destroyAllLinks()
  GN[gn][673] = 1  // mark genome in use

  if parent == -1:
      restTime--
      if move && restTime > 0:
          moveSeed()
      if restTime <= 0:
          age = AGE
          aGen = 0
          type = APEX
          level = 0
  ```

---

### Phase 6: Death and Recycling

- [ ] **6.1 — Death recycling** (`src/cells/systems.rs`)

  Full death sequence (`Cells.pde:1036-1058`):
  1. `transmitEnergy()` — try to send energy to connected cells first
  2. `distributeOrganic(X, Y, org)` — spread cell's organic (15) to 3x3 soil
  3. `distributeZarad(X, Y, energy+engP+engM)` — spread remaining energy to 3x3 soil
  4. Return cell to free pool: `freeCells[freeCellsPointer] = index`
  5. Reset all fields, remove from linked list

  Note: `distributeOrganic` and `distributeZarad` **average with existing soil values**
  (they sum the 3x3 + deposit, then divide by 9 and redistribute). They do NOT simply
  add to existing values.

- [ ] **6.2 — Orphan death** (`src/cells/systems.rs`)

  LEAF/ROOT/ANTN die if `parent == -1`:
  - LEAF: `Cells.pde:120` (`if(age <= 0 || parent == -1) die()`)
  - ROOT: `Cells.pde:140` (same)
  - ANTN: `Cells.pde:160` (same)

- [ ] **6.3 — Isolated Branch death** (`src/cells/systems.rs`)

  WOOD: `Cells.pde:99-101`:

  ```
  if parent == -1:
      has_children = children[0] + children[1] + children[2] + children[3]
      if has_children == 0: die()
  ```

---

### Phase 7: Mutation

- [ ] **7.1 — Mutation on APEX child creation** (`src/genes.rs`)

  Original (`Cells.pde:314-323`):

  ```
  if stg == APEX && freeGNPointer < TOTAL_GENOM_COUNT - 1000:
      if random(0, 100) <= 1:  // ~1% chance
          g = findFreeGenom()
          arrayCopy(GN[gn], GN[g])     // clone genome
          mutGen = floor(random(0, 673))  // pick random byte
          GN[g][mutGen] = floor(random(0, 256))  // randomize it
          GN[g][673] = 1  // mark in use
          cells[newIndex].gn = g
  ```

  In Rust, since genomes are per-entity `Component`s (not a global pool), there's no
  genome count limit. The 1% check applies. "One byte change" maps to: pick a random
  `GenomeEntry` (0-51), pick a random field within it (spawn direction, precondition,
  action, genome pointer, command), and randomize that field.

- [ ] **7.2 — `Genome::mutate()` method** (`src/genes.rs`)

  Design decision: the original's 673 bytes correspond to 32 genes × 21 bytes. In Rust's
  52-entry `GenomeEntry` struct, each entry has:
  - `spawn: GenomeSpawn` (3 fields: forward/left/right cell types)
  - `conditionals: GenomeConditional`:
    - `preconditions: Vec<GenomePrecondition>` (0-2 entries)
    - `preconditions_met_action: CellAction` (Command or ChangeGenome)
    - `preconditions_unmet_action: CellAction`
    - `fallback_genome: GenomeID`

  A reasonable mapping: pick a random entry, pick a random sub-field, generate a new
  random value for that field using the existing `Distribution<T>` impls (which are
  already defined for all types).

---

### Phase 8: Initialization

- [ ] **8.1 — Fix `initialize_sprouts_system`** (`src/main.rs:247-274`)

  Original `createNewLife()` (`func.pde:3-12`):

  ```
  for x = 10; x < W; x += 20:
      for y = 10; y < H; y += 20:
          createCell(x, y, APEX, 500 energy, random genome, unique adam)
  ```

  Current Rust version has bugs:
  - All sprouts share ONE genome (line 252: genome generated once)
  - Energy is 10 instead of 500
  - Positions are random instead of grid-aligned every 20 cells

- [ ] **8.2 — Adjust initial energy values** (`src/main.rs:130,134`)

  Original: `setOrganicZarad(200, 200)` (`CLANS3eng.pde:83`, `func.pde:32-40`).
  Current Rust: organic=50, charge=20.

- [ ] **8.3 — Remove `add_test_cells`** (`src/main.rs:351-411`)

  Replace with real initialization.

- [ ] **8.4 — Wire CLI parsing** (`src/cli.rs`, `src/main.rs`)

  `Cli` struct exists but is unused.

---

### Phase 9: Ancestor/Clan Tracking

- [ ] **9.1 — `CellAncestor` component** (`src/cells/mod.rs`)

  `usize`. Each initial cell gets unique ID. Children inherit parent's ancestor.

  Original: `adam` field, set during `createNewLife()` (`func.pde:9`: incrementing counter)
  and inherited during growth (`Cells.pde:303`: `adam = parent.adam`).

  Used for lineage visualization and statistics: `func.pde:217-231` counts cells per
  ancestor.

---

### Phase 10: Verification

- [ ] **10.1** Unit test: growth produces correct child cells from `GenomeSpawn`
- [ ] **10.2** Unit test: ping-pong energy transfer moves energy 1 cell/tick
- [ ] **10.3** Unit test: death recycling distributes correct amounts to 9-cell
      neighborhood
- [ ] **10.4** Unit test: solar formula matches original (organic × free_count × 0.0008,
      zero if adjacent leaf)
- [ ] **10.5** Unit test: seed movement + collision behavior
- [ ] **10.6** Unit test: mutation produces genome with exactly 1 difference
- [ ] **10.7** Integration test: single APEX with known genome grows correctly over N
      ticks
- [ ] **10.8** Verify all single-cell and multi-cell commands emit correct requests

---

## Recommended Implementation Order

1. **Phase 0** — foundation everything depends on
2. **Phase 1** — sets the architectural pattern
3. **Phase 2** — energy is critical for cell survival
4. **Phase 6** — closes the resource loop
5. **Phase 3** — core genome mechanics
6. **Phase 5** — reproduction
7. **Phase 7** — evolution
8. **Phase 4** — richer behavior
9. **Phase 8** — real simulation runs
10. **Phase 9** — visualization
11. **Phase 10** — verification throughout, especially at end

## Critical Files

| File                    | Role                                                         |
| ----------------------- | ------------------------------------------------------------ |
| `src/cells/systems.rs`  | Genome execution, command implementations, resolve systems   |
| `src/energy/mod.rs`     | Energy environment methods, transfer buffers, new components |
| `src/energy/systems.rs` | Energy collection, transfer, costs                           |
| `src/genes.rs`          | Precondition expansion, mutation, genome execution           |
| `src/main.rs`           | System sets, initialization, ordering                        |
| `src/simulation.rs`     | SimulationGrid as live spatial index                         |
| `src/cells/mod.rs`      | New components (Age, Level, Organic, Ancestor, requests)     |

## Quick Reference: Original Constants

From `constant.pde`:

| Constant          | Value  | Used In                          |
| ----------------- | ------ | -------------------------------- |
| `ROOT_CAN`        | 1.0    | ROOT extraction per tick         |
| `ANTN_CAN`        | 1.0    | ANTN extraction per tick         |
| `ALONE_CAN`       | 6      | Lone APEX soil absorption        |
| `ORGANIC_EXCESS`  | 512    | Poison threshold (organic)       |
| `ENERGY_EXCESS`   | 512    | Poison threshold (energy)        |
| `AGE`             | 3      | Default cell lifespan            |
| `Energy4Life`     | 0.04   | WOOD/LEAF/ROOT/ANTN cost/tick    |
| `SeedEnergy4Life` | 0.5    | Seed cost/tick                   |
| `ApexEnergy4Life` | 1.0    | APEX/Sprout cost/tick            |
| `moveApexPrice`   | 1.0    | Movement cost                    |
| `LIGHTENERGY`     | 10     | Leaf base light factor           |
| `LIGHTCOEF`       | 0.0008 | Leaf light coefficient           |
| `ORGANIC_CELL`    | 15     | Organic per cell (death release) |
| `WORK`            | 5      | Energy cost to create cell       |
| `MAX_APEX_ENERGY` | 1024   | APEX detach threshold            |
| `MAX_SEED_ENERGY` | 512    | Seed detach threshold            |
