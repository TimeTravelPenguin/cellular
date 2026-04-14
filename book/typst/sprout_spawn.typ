#import "lib.typ": cell-colours
#import "@preview/cetz:0.4.2"

#let GRID_STEP = 1
#let CELL_RADIUS = 0.6
#let BRANCH_THICKNESS = 0.1

#let pos(x, y) = ((x + 0.5) * GRID_STEP, (y + 0.5) * GRID_STEP)

#let sprout(x, y, radius: CELL_RADIUS, ..args) = {
  import cetz.draw: *

  circle(
    pos(x, y),
    radius: radius / 2,
    fill: cell-colours.sprout,
    ..args,
  )
}

#let antenna(x, y, radius: CELL_RADIUS, ..args) = {
  import cetz.draw: *

  circle(
    pos(x, y),
    radius: radius / 2,
    fill: cell-colours.antenna,
    ..args,
  )
}

#let root(x, y, radius: CELL_RADIUS, ..args) = {
  import cetz.draw: *

  rect(
    pos(x - CELL_RADIUS / 2, y - CELL_RADIUS / 2),
    pos(x + CELL_RADIUS / 2, y + CELL_RADIUS / 2),
    fill: cell-colours.root,
  )
}

#let leaf(x, y, vert: true, ..args) = {
  import cetz.draw: *

  let rx = CELL_RADIUS * 0.4
  let ry = CELL_RADIUS * 0.7

  if not vert {
    (rx, ry) = (ry, rx)
  }

  circle(
    pos(x, y),
    radius: (rx, ry),
    fill: cell-colours.leaf,
    ..args,
  )
}

#let remove(x, y, cells) = {
  let ys = cells.at(str(x), default: (:))
  _ = ys.remove(str(y))
  cells.insert(str(x), ys)
  cells
}

#let get(x, y, cells, default: none) = {
  let ys = cells.at(str(x), default: (:))
  ys.at(str(y), default: default)
}

#let insert(x, y, cell, cells) = {
  let ys = cells.at(str(x), default: (:))
  ys.insert(str(y), cell)
  cells.insert(str(x), ys)
  cells
}

#let move(from, to, cells) = {
  let cell = get(..from, cells)
  let cells = remove(..from, cells)
  if cell != none {
    insert(..to, cell, cells)
  } else {
    cells
  }
}

#let _add(cells, x, y, cell) = {
  insert(x, y, cell, cells)
}

#let add-sprout(x, y, cells) = {
  _add(cells, x, y, "sprout")
}

#let add-leaf(x, y, cells) = {
  _add(cells, x, y, "leaf")
}

#let add-root(x, y, cells) = {
  _add(cells, x, y, "root")
}

#let add-antenna(x, y, cells) = {
  _add(cells, x, y, "antenna")
}

#let add-branch(x, y, to, cells) = {
  let branches = cells.branches
  let children = get(x, y, branches, default: ())

  if type(to) == array and to.all(x => type(x) != array) {
    children.push(to)
  } else {
    children += to
  }

  let branches = insert(x, y, children, branches)
  cells.insert("branches", branches)
  cells
}

#let draw(cells) = {
  import cetz.draw: *

  for (x, ys) in cells.branches.pairs() {
    for (y, children) in ys.pairs() {
      let from = pos(float(x), float(y))
      for (to_x, to_y) in children {
        let to = pos(float(to_x), float(to_y))
        line(
          from,
          to,
          stroke: (
            paint: cell-colours.branch,
            cap: "square",
          ),
        )
      }
    }
  }

  for (x, ys) in cells.pairs() {
    if x == "branches" {
      continue
    }

    for (y, cell) in ys.pairs() {
      let (x, y) = (float(x), float(y))
      if cell == "sprout" {
        sprout(x, y)
      } else if cell == "leaf" {
        leaf(x, y)
      } else if cell == "root" {
        root(x, y)
      } else if cell == "antenna" {
        antenna(x, y)
      } else {
        panic("Unknown cell type: " + cell)
      }
    }
  }
}

#let in-canvas(cells) = {
  cetz.canvas({
    import cetz.draw: *

    grid(
      (0, -1),
      (3, 2),
      step: GRID_STEP,
      help-lines: true,
    )

    draw(cells)
  })
}

#let cells = (branches: (:))
#let frames = ()

#let cells = add-sprout(0, 0, cells)
#frames.push(in-canvas(cells))

#let cells = (
  move.with((0, 0), (1, 0)),
  add-branch.with(0, 0, ((1, 0), (0, 1))),
  add-leaf.with(0, 1),
).fold(cells, (c, action) => action(c))
#frames.push(in-canvas(cells))

#let cells = (
  move.with((1, 0), (2, 0)),
  add-root.with(1, 1),
  add-branch.with(1, 0, ((2, 0), (1, 1))),
).fold(cells, (c, action) => action(c))
#frames.push(in-canvas(cells))

#let cells = (
  move.with((2, 0), (2, -1)),
  add-antenna.with(2, 1),
  add-branch.with(2, 0, ((2, -1), (2, 1))),
).fold(cells, (c, action) => action(c))
#frames.push(in-canvas(cells))

#set page(height: auto, width: auto, margin: 1cm)
#set text(font: "Nunito Sans 7pt")
#show grid.cell: set align(horizon)

#grid(
  columns: 2 * frames.len() - 1,
  column-gutter: 1em,
  ..frames
    .zip((
      "Spawn Sprout",
      "Sprout Spawns Leaf",
      "Sprout Spawns Root",
      "Sprout Spawns Antenna",
    ).map(text.with(size: 7pt)))
    .map(((x, y)) => stack(spacing: 1em, y, x))
    .intersperse($=>$)
)
