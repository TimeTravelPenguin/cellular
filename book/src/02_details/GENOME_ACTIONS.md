# Genome Actions Reference

Detailed breakdown of all genome-driven actions in the CLANS3 simulation, extracted from
the Processing source code (`Cells.pde`, `constant.pde`).

---

## Overview

Only **APEX (Sprout)** cells execute genome commands. The genome system has three
execution paths depending on cell state:

1. **Growth** — when no conditions are defined in the active gene
2. **Body commands** — when the APEX has a parent (is part of a multicellular organism)
3. **Lone commands** — when the APEX has no parent (is a free-roaming single cell)

---

## Execution Flow (per tick)

```
APEX evaluates active gene:
  ├─ Check condition 1 (bytes 3-4)
  ├─ Check condition 2 (bytes 5-6)
  │
  ├─ Both conditions unused (gene value > 67)?
  │   └─ GROW using bytes 0-2 → APEX becomes WOOD
  │
  ├─ At least one condition met (none return -1)?
  │   ├─ Has parent? → execute body command (byte 9)
  │   │   └─ success → gene byte 10, failure → gene byte 11
  │   ├─ No parent? → execute lone command (byte 15)
  │   │   └─ success → gene byte 16, failure → gene byte 17
  │   └─ No valid command? → jump to gene byte 7
  │
  └─ At least one condition NOT met?
      ├─ Has parent? → execute body command (byte 12)
      │   └─ success → gene byte 13, failure → gene byte 14
      ├─ No parent? → execute lone command (byte 18)
      │   └─ success → gene byte 19, failure → gene byte 20
      └─ No valid command? → jump to gene byte 8
```

---

## Growth (No Conditions Active)

**Who:** APEX cells (both body and lone) **When:** Both condition slots in the active gene
have values > 67 (maxIF) **Effect:** The APEX creates up to 3 new cells
(left/forward/right) based on bytes 0-2, then transforms into WOOD.

### Growth Byte Encoding (each of bytes 0, 1, 2)

| Value Range | Cell Created | Notes                       |
| ----------- | ------------ | --------------------------- |
| 0-63        | APEX         | Active gene = value % 32    |
| 64-75       | LEAF         |                             |
| 76-85       | ANTN         |                             |
| 86-95       | ROOT         |                             |
| 96-255      | Nothing      | No growth in this direction |

### Growth Requirements

- Energy cost: **(WORK + ORGANIC_CELL) = 20** energy per new cell
- Total energy must be >= `cell_count * 20` or growth is skipped entirely
- Target position must be empty (cellsIndx == 0), otherwise that branch is silently skipped
- The growing APEX becomes WOOD after growth
- If no branches can be created and cell is alone, active gene resets to 0

### Growth Side Effects

- New APEX children: parent WOOD sets `energyTo[]` flag toward them (will send energy)
- New LEAF/ROOT/ANTN children: child sets `energyTo[]` flag toward parent (will send energy back)
- Mutation: ~1% chance per new APEX child — copies genome, changes 1 random byte

---

## Body Commands (APEX with parent, gene value 0-14)

These commands are available when the APEX is part of a multicellular organism (has a parent).

| Cmd | Name                   | Description                          | Returns        | Details                                                                                                                                             |
| --- | ---------------------- | ------------------------------------ | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0   | **Skip**               | Do nothing                           | `true`         | No-op                                                                                                                                               |
| 1   | **Fly Seed**           | Transform into flying seed           | `true`         | Sets type=SEED, calls `destroyAllLinks()`, move=true, restTime=8                                                                                    |
| 2   | **Stationary Seed**    | Transform into stationary seed       | `true`         | Sets type=SEED, move=false, restTime=8                                                                                                              |
| 3   | **Delayed Fly Seed**   | Transform into seed that flies later | `true`         | Sets type=SEED, move=true, restTime=8. Note: does NOT call destroyAllLinks() — stays attached until detach threshold                                |
| 4   | **Die**                | Voluntary death                      | `true`         | Sets `energyTo[parent]=1` before dying (tries to send energy to parent), then calls `die()`                                                         |
| 5   | **Detach**             | Break away from parent               | `true`         | Calls `destroyAllLinks()` — severs all parent/child/energy connections                                                                              |
| 6   | **Push Energy Left**   | Move soil energy to left             | `true`         | Transfers all EnergyMap at cell position to the cell one step left (relative to facing)                                                             |
| 7   | **Push Energy Right**  | Move soil energy to right            | `true`         | Same as above, rightward                                                                                                                            |
| 8   | **Push Energy Ahead**  | Move soil energy forward             | `true`         | Same as above, forward                                                                                                                              |
| 9   | **Push Organic Left**  | Move soil organic to left            | `true`         | Transfers all OrganicMap at cell position to cell one step left                                                                                     |
| 10  | **Push Organic Right** | Move soil organic to right           | `true`         | Same, rightward                                                                                                                                     |
| 11  | **Push Organic Ahead** | Move soil organic forward            | `true`         | Same, forward                                                                                                                                       |
| 12  | **Fire**               | Launch bullet projectile             | `true`/`false` | Creates a SEED with 30 energy, restTime=30 ticks, move=true. Costs ORGANIC_CELL+WORK+30 = 50 energy. Fails if insufficient energy or target is wall |
| 13  | **Seed**               | Create reproductive seed             | `true`/`false` | Creates SEED with most of parent's energy, restTime=5+random(40), move=true. Parent keeps 30 energy. Costs ORGANIC_CELL+WORK+30 = 50 minimum        |
| 14  | **Scatter Organic**    | Convert energy to soil organic       | `true`         | Distributes `floor((energy-3)/9)` organic to each of 9 surrounding cells. Cell retains 3 energy. Skipped if energy < 12                             |

### Fire/Seed Collision Behavior (setNewSEED)

When the target position (one step ahead) is occupied:

- If occupied by **same genome** → adds (ORGANIC_CELL+WORK+30) energy to that cell
- If occupied by **different genome** → marks that cell for death
- Either way, costs the creating cell (ORGANIC_CELL+WORK+30) energy
- If target is a wall (cellsIndx < 0) → fails, returns false

---

## Lone Commands (APEX without parent, gene value 0-17)

These commands are available when the APEX is a free-roaming single cell (no parent).

| Cmd | Name                        | Description                    | Returns        | Details                                                                                                                                                                                                  |
| --- | --------------------------- | ------------------------------ | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0   | **Move Forward**            | Step one cell ahead            | `true`/`false` | Costs 1 energy (moveApexPrice). Fails if target occupied (any cell or wall)                                                                                                                              |
| 1   | **Turn Right**              | Rotate 90° clockwise           | `true`         | direction += 1 (mod 4)                                                                                                                                                                                   |
| 2   | **Turn Left**               | Rotate 90° counter-clockwise   | `true`         | direction -= 1 (mod 4)                                                                                                                                                                                   |
| 3   | **Turn Around**             | Rotate 180°                    | `true`         | direction += 2 (mod 4)                                                                                                                                                                                   |
| 4   | **Turn Right + Move**       | Rotate 90° CW then step        | `true`/`false` | Turn always succeeds, move may fail                                                                                                                                                                      |
| 5   | **Turn Left + Move**        | Rotate 90° CCW then step       | `true`/`false` | Same                                                                                                                                                                                                     |
| 6   | **Turn Around + Move**      | Rotate 180° then step          | `true`/`false` | Same                                                                                                                                                                                                     |
| 7   | **Parasite**                | Attach to adjacent WOOD cell   | `true`/`false` | Checks cell ahead: if it's WOOD, sets self.parent to that direction and registers on the WOOD's energyTo/children arrays. Fails if no WOOD ahead                                                         |
| 8   | **Random Turn**             | Turn randomly                  | `true`         | 30% right, 30% left, 40% no turn                                                                                                                                                                         |
| 9   | **Random Turn + Move**      | Random turn then step          | `true`/`false` | Same random turn logic, then moveApex()                                                                                                                                                                  |
| 10  | **Drag Organic from Left**  | Pull organic under self        | `true`/`false` | Moves all OrganicMap from left cell to current position. Returns false if source had 0 organic                                                                                                           |
| 11  | **Drag Organic from Ahead** | Pull organic under self        | `true`/`false` | Same, from ahead                                                                                                                                                                                         |
| 12  | **Drag Organic from Right** | Pull organic under self        | `true`/`false` | Same, from right                                                                                                                                                                                         |
| 13  | **Drag Energy from Left**   | Pull energy under self         | `true`/`false` | Moves all EnergyMap from left cell to current position. Returns false if source had 0 energy                                                                                                             |
| 14  | **Drag Energy from Ahead**  | Pull energy under self         | `true`/`false` | Same, from ahead                                                                                                                                                                                         |
| 15  | **Drag Energy from Right**  | Pull energy under self         | `true`/`false` | Same, from right                                                                                                                                                                                         |
| 16  | **Eat Neighbors**           | Kill and absorb adjacent cells | `true`/`false` | Costs 1 energy. Checks all 8 directions (cardinal + diagonal). Kills all cells with type < WOOD (i.e., APEX, LEAF, ANTN, ROOT). Absorbs their energy + engP + engM + org. Returns false if nothing eaten |
| 17  | **Absorb Soil Energy**      | Extract energy from ground     | `true`/`false` | Takes up to ALONE_CAN=6 energy from EnergyMap at current position. Returns false if soil had less than 6 energy (still takes what's there)                                                               |

---

## Conditions (68 total, IDs 0-67)

Conditions are checked before commands execute. Each gene has two condition slots (bytes 3-4 and 5-6). A condition slot is "unused" if the gene value > 67 (maxIF).

### Return Values

- `1` — condition met
- `-1` — condition not met
- `0` — condition slot unused (gene value > maxIF)

### Condition Table

| ID                                                                      | Condition                           | Parameter Usage                                                                                                    |
| ----------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Resource at Position**                                                |                                     |                                                                                                                    |
| 0                                                                       | Organic at cell < threshold         | param \* 2                                                                                                         |
| 1                                                                       | Organic at cell >= threshold        | param \* 2 (NOTE: source code is identical to 0 — likely a bug, SIMULATION.md says >=)                             |
| 2                                                                       | Cell energy > threshold             | param \* 2                                                                                                         |
| 3                                                                       | Cell energy < threshold             | param \* 2                                                                                                         |
| **Level-Based**                                                         |                                     |                                                                                                                    |
| 4                                                                       | param % (level+1) == 0              | param value directly                                                                                               |
| 5                                                                       | level % (param+1) == 0              | param value directly                                                                                               |
| 6                                                                       | level > param                       | param value directly                                                                                               |
| 7                                                                       | level < param                       | param value directly                                                                                               |
| **Energy Trends**                                                       |                                     |                                                                                                                    |
| 8                                                                       | Energy rising (current >= previous) | —                                                                                                                  |
| 9                                                                       | Energy falling (current < previous) | —                                                                                                                  |
| **Area Resources (9 cells around position)**                            |                                     |                                                                                                                    |
| 10                                                                      | Organic(9) > threshold              | param \* 18                                                                                                        |
| 11                                                                      | Organic(9) < threshold              | param \* 18                                                                                                        |
| 12                                                                      | Energy(9) > threshold               | param \* 18                                                                                                        |
| 13                                                                      | Energy(9) < threshold               | param \* 18                                                                                                        |
| 14                                                                      | Energy(9) > Organic(9)              | —                                                                                                                  |
| 15                                                                      | Energy(9) < Organic(9)              | —                                                                                                                  |
| **Spatial Awareness**                                                   |                                     |                                                                                                                    |
| 16                                                                      | Edible cells nearby                 | Checks 5 directions (left, front-left, front, front-right, right). "Edible" = type < WOOD (APEX, LEAF, ANTN, ROOT) |
| 17                                                                      | Area free (left + center + right)   | All three relative directions must be empty                                                                        |
| 18                                                                      | Free left                           | —                                                                                                                  |
| 19                                                                      | Free center (ahead)                 | —                                                                                                                  |
| 20                                                                      | Free right                          | —                                                                                                                  |
| 21                                                                      | Obstacle left                       | —                                                                                                                  |
| 22                                                                      | Obstacle center (ahead)             | —                                                                                                                  |
| 23                                                                      | Obstacle right                      | —                                                                                                                  |
| 24                                                                      | Has parent                          | —                                                                                                                  |
| 25                                                                      | Random                              | true if random(256) > param                                                                                        |
| **Light Comparisons (organic at 3 cells distance, excluding poisoned)** |                                     |                                                                                                                    |
| 26                                                                      | Light center > light right          | —                                                                                                                  |
| 27                                                                      | Light right > light center          | —                                                                                                                  |
| 28                                                                      | Light center > light left           | —                                                                                                                  |
| 29                                                                      | Light left > light center           | —                                                                                                                  |
| 30                                                                      | Light left > light right            | —                                                                                                                  |
| 31                                                                      | Light right > light left            | —                                                                                                                  |
| **Energy(9) Directional Comparisons**                                   |                                     |                                                                                                                    |
| 32                                                                      | Energy(9) center > right            | —                                                                                                                  |
| 33                                                                      | Energy(9) right > center            | —                                                                                                                  |
| 34                                                                      | Energy(9) center > left             | —                                                                                                                  |
| 35                                                                      | Energy(9) left > center             | —                                                                                                                  |
| 36                                                                      | Energy(9) left > right              | —                                                                                                                  |
| 37                                                                      | Energy(9) right > left              | —                                                                                                                  |
| 38                                                                      | Energy(9) right > threshold         | param \* 18                                                                                                        |
| 39                                                                      | Energy(9) center > threshold        | param \* 18                                                                                                        |
| 40                                                                      | Energy(9) left > threshold          | param \* 18                                                                                                        |
| **Organic(9) Directional Comparisons**                                  |                                     |                                                                                                                    |
| 41                                                                      | Organic(9) center > right           | —                                                                                                                  |
| 42                                                                      | Organic(9) right > center           | —                                                                                                                  |
| 43                                                                      | Organic(9) center > left            | —                                                                                                                  |
| 44                                                                      | Organic(9) left > center            | —                                                                                                                  |
| 45                                                                      | Organic(9) left > right             | —                                                                                                                  |
| 46                                                                      | Organic(9) right > left             | —                                                                                                                  |
| 47                                                                      | Organic(9) center > threshold       | param \* 18                                                                                                        |
| 48                                                                      | Organic(9) right > threshold        | param \* 18                                                                                                        |
| 49                                                                      | Organic(9) left > threshold         | param \* 18                                                                                                        |
| **Free Space(9) Directional Comparisons**                               |                                     |                                                                                                                    |
| 50                                                                      | Free(9) center > right              | —                                                                                                                  |
| 51                                                                      | Free(9) right > center              | —                                                                                                                  |
| 52                                                                      | Free(9) center > left               | —                                                                                                                  |
| 53                                                                      | Free(9) left > center               | —                                                                                                                  |
| 54                                                                      | Free(9) left > right                | —                                                                                                                  |
| 55                                                                      | Free(9) right > left                | —                                                                                                                  |
| 56                                                                      | Free(9) center > threshold          | param % 10                                                                                                         |
| 57                                                                      | Free(9) right > threshold           | param % 10                                                                                                         |
| 58                                                                      | Free(9) left > threshold            | param % 10                                                                                                         |
| **Poison Detection**                                                    |                                     |                                                                                                                    |
| 59                                                                      | Organic poison ahead                | OrganicMap >= 512 one step ahead                                                                                   |
| 60                                                                      | Organic poison left                 | OrganicMap >= 512 one step left                                                                                    |
| 61                                                                      | Organic poison right                | OrganicMap >= 512 one step right                                                                                   |
| 62                                                                      | Energy poison ahead                 | EnergyMap >= 512 one step ahead                                                                                    |
| 63                                                                      | Energy poison left                  | EnergyMap >= 512 one step left                                                                                     |
| 64                                                                      | Energy poison right                 | EnergyMap >= 512 one step right                                                                                    |
| 65                                                                      | Any poison ahead                    | Either map >= 512 one step ahead                                                                                   |
| 66                                                                      | Any poison left                     | Either map >= 512 one step left                                                                                    |
| 67                                                                      | Any poison right                    | Either map >= 512 one step right                                                                                   |

---

## Non-Genome Cell Behaviors (Automatic, Every Tick)

These are not genome-driven — they happen automatically based on cell type.

### SEED

- Costs 0.5 energy/tick
- Dies if energy < 0
- If energy > 512 while attached → detaches (`destroyAllLinks()`)
- If detached and restTime > 0: counts down, moves 1 cell/tick if `move=true`
  - **Collision:** kills the hit cell, seed stops moving (move=false, returns false → seed dies)
  - Movement costs 1 energy/step
- When restTime reaches 0: transforms into APEX at gene 0, level=0, age=AGE

### WOOD (Transport)

- Costs 0.04 energy/tick
- If energy > 0: transmits energy to all cells marked in `energyTo[]`, split evenly
- If energy <= 0: loses 1 age tick (starts at AGE=3)
- Dies when age reaches 0
- If detached (no parent) and has no children → dies immediately

### LEAF (Green)

- **Photosynthesis:** `OrganicMap[x][y] * free_neighbor_count * 0.0008`
  - free_neighbor_count starts at LIGHTENERGY=10
  - Each occupied neighbor (8 checked): -1 from count
  - If ANY adjacent cell is LEAF: returns **0** (mutual shading — complete shutdown)
- Costs 0.04 energy/tick
- Transmits surplus energy toward parent
- Dies when age reaches 0 OR if parent == -1 (detached)

### ROOT (Red)

- Extracts up to ROOT_CAN=1 organic/tick from OrganicMap at position
- **Immune to organic poisoning** (can survive OrganicMap >= 512)
- Costs 0.04 energy/tick
- Transmits surplus energy toward parent
- Dies when age reaches 0 OR if detached

### ANTN (Blue)

- Extracts up to ANTN_CAN=1 energy/tick from EnergyMap at position
- **Immune to energy poisoning** (can survive EnergyMap >= 512)
- Costs 0.04 energy/tick
- Transmits surplus energy toward parent
- Dies when age reaches 0 OR if detached

### APEX (Sprout)

- Costs 1 energy/tick
- Dies if energy + engP + engM < 0
- If energy > 1024 while attached → detaches and resets to gene 0
- If detached → level resets to 0

---

## Energy Transfer System

- Uses double-buffer (`engP`/`engM`) with alternating phase (`EnergyTransportPeriod` flips between +1/-1 each tick)
- Cells receive buffered energy at start of tick
- `transmitEnergy()`: splits cell's energy evenly among all flagged `energyTo[]` directions
- If nowhere to send and has parent: redirects to parent, also tells parent to stop sending energy back
- If nowhere to send and no parent: dumps energy into EnergyMap, loses 1 age tick

---

## Key Constants

| Constant        | Value  | Used By                                                         |
| --------------- | ------ | --------------------------------------------------------------- |
| ROOT_CAN        | 1      | ROOT extraction rate                                            |
| ANTN_CAN        | 1      | ANTN extraction rate                                            |
| ALONE_CAN       | 6      | Lone APEX soil energy absorption                                |
| ORGANIC_EXCESS  | 512    | Poison threshold (organic)                                      |
| ENERGY_EXCESS   | 512    | Poison threshold (energy)                                       |
| AGE             | 3      | Base lifespan ticks (when no energy)                            |
| Energy4Life     | 0.04   | LEAF/ROOT/ANTN/WOOD living cost                                 |
| SeedEnergy4Life | 0.5    | SEED living cost                                                |
| ApexEnergy4Life | 1      | APEX living cost                                                |
| moveApexPrice   | 1      | APEX movement cost                                              |
| LIGHTENERGY     | 10     | Base free-neighbor count for photosynthesis                     |
| LIGHTCOEF       | 0.0008 | Photosynthesis coefficient                                      |
| ORGANIC_CELL    | 15     | Organic deposited into soil on death; organic cost per new cell |
| WORK            | 5      | Energy cost of creating a cell                                  |
| MAX_APEX_ENERGY | 1024   | APEX detach threshold                                           |
| MAX_SEED_ENERGY | 512    | SEED detach threshold                                           |

---

## Source Code Notes

- **Bug in condition 1:** The code for conditions 0 and 1 is identical (`< param*2`). `SIMULATION.md` describes condition 1 as `>=`, so this appears to be a copy-paste bug in the original Processing source.
- **"Light" conditions (26-31):** "Light" actually measures organic matter at 3 cells distance in the given direction, excluding poisoned cells (organic >= 512 counts as 0). This is because photosynthesis depends on soil organic.
- **Directional (9) scans (32-58):** These sample a 3x3 grid offset 2 cells in the given direction, not the 3x3 around the cell itself.
- **Eat (lone cmd 16):** Checks all 8 neighbors (4 cardinal + 4 diagonal via 0.5-step increments). Only eats types < WOOD (APEX=0, LEAF=1, ANTN=2, ROOT=3). WOOD and SEED are spared.
- **Command 3 vs Command 1:** Both set move=true and restTime=8, but only command 1 calls `destroyAllLinks()`. Command 3 stays attached, meaning it won't actually fly until either energy exceeds MAX_SEED_ENERGY (512) causing auto-detach, or it dies.
