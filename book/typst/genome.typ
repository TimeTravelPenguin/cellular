#import "lib.typ": genome-colours
#import "@preview/shadowed:0.3.0": shadow
#import "@preview/cetz:0.4.2"

#let BLOCK-TEXT-SIZE = 0.5cm
#let BLOCK-CMD-TEXT-SIZE = 0.45cm
#let BLOCK-SIZE = 1cm
#let BLOCK-STROKE = (
  paint: black,
  thickness: 0.7pt,
  cap: "round",
  join: "round",
)

#let block-args = (
  height: BLOCK-SIZE,
  width: BLOCK-SIZE,
  stroke: BLOCK-STROKE,
  radius: 0.7pt,
)

#let shadow-args = (
  blur: 5pt,
  fill: rgb(89, 85, 101, 50%),
  radius: 4pt,
)

#let genome(shadows: true, ..args) = {
  let inner = square(
    ..block-args,
    ..args,
  )

  if shadows {
    shadow(
      ..shadow-args,
      inner,
    )
  } else {
    inner
  }
}

#let spawn(shadows: true) = genome(shadows: shadows, fill: genome-colours.spawn)

#let conditional(shadows: true) = genome(
  shadows: shadows,
  fill: genome-colours.conditional,
  text("if", fill: genome-colours.conditional-text),
)

#let predicate(shadows: true) = genome(
  shadows: shadows,
  fill: genome-colours.predicate,
  text("P", fill: genome-colours.predicate-text),
)

#let command(shadows: true) = genome(
  shadows: shadows,
  fill: genome-colours.command,
  text("cmd", fill: genome-colours.command-text, size: BLOCK-CMD-TEXT-SIZE),
)

#let fallback(shadows: true) = genome(
  shadows: shadows,
  fill: genome-colours.fallback,
  text("aG", fill: genome-colours.fallback-text),
)

#set page(height: auto, width: auto, margin: 1cm)

#show square: it => {
  set text(
    font: "Fira Code",
    weight: "bold",
    size: BLOCK-TEXT-SIZE,
  )

  set align(center + horizon)
  it
}

#let genome-group(n, space: 1.5mm) = (space,) * (n - 1)

#let genome-groups = (
  spawn-actions: (spawn(),) * 3,
  condition: (conditional(), predicate()),
  fallback: (fallback(),) * 2,
  command: (command(), fallback(), fallback()),
)

#let genome-sequence = (
  genome-groups.spawn-actions,
  genome-groups.condition,
  genome-groups.condition,
  genome-groups.fallback,
  genome-groups.command,
  genome-groups.command,
  genome-groups.command,
  genome-groups.command,
)

#let sequence-spacing = (
  genome-group(3),
  genome-group(2),
  genome-group(2),
  genome-group(2),
  genome-group(3),
  genome-group(3),
  genome-group(3),
  genome-group(3),
)

#grid(
  columns: 21,
  column-gutter: sequence-spacing.intersperse(5mm).flatten(),
  ..genome-sequence.flatten(),
)

#pagebreak()

#let labels = (
  "Spawn Actions",
  "Condition 01",
  "Condition 02",
  "Command Fallbacks",
  "Sprout Command 01",
  "Sprout Command 02",
  "Seed Command 01",
  "Seed Command 02",
)

#let indexes = (
  (0, 1, 2),
  (3, 4),
  (5, 6),
  (7, 8),
  (9, 10, 11),
  (12, 13, 14),
  (15, 16, 17),
  (18, 19, 20),
)

#stack(
  dir: ltr,
  spacing: 5mm,
  ..genome-sequence
    .zip(sequence-spacing, indexes)
    .map(((group, spacing, indexes)) => grid(
      columns: group.len(),
      column-gutter: spacing,
      ..group
        .zip(indexes)
        .map(((block, index)) => stack(
          dir: ttb,
          spacing: 1mm,
          align(center, text(str(index), font: "Fira Code", size: 7pt)),
          block,
        )),
    ))
    .flatten(),
)

#pagebreak()

#stack(
  dir: ltr,
  spacing: 5mm,
  ..genome-sequence
    .zip(sequence-spacing, labels)
    .map(((group, spacing, label)) => math.underbrace(
      grid(
        columns: group.len(),
        column-gutter: spacing,
        ..group,
      ),
      text(label, font: "Nunito Sans 7pt", size: 7pt),
    ))
    .flatten(),
)

#pagebreak()

#grid(
  columns: 21,
  row-gutter: 2mm,
  column-gutter: sequence-spacing.intersperse(5mm).flatten(),
  ..((genome-sequence,) * 52).flatten(),
)


#pagebreak()

#cetz.canvas(
  // debug: true,
  {
    import cetz.draw: *

    let indexed-content(blocks, indices) = stack(
      dir: ltr,
      spacing: 2mm,
      ..blocks
        .zip(indices)
        .map(((block, index)) => stack(
          dir: ttb,
          spacing: 1mm,
          align(center, text(str(index), font: "Fira Code", size: 7pt)),
          block,
        )),
    )

    let conditional-content(indices) = indexed-content(
      genome-groups.condition,
      indices,
    )

    let spawn-content = indexed-content(
      genome-groups.spawn-actions,
      (0, 1, 2),
    )

    let title-block(title, ..args) = align(center, stack(
      dir: ttb,
      spacing: 3mm,
      text(
        title,
        font: "Nunito Sans 7pt",
        fill: gray,
        size: 7pt,
      ),
      shadow(
        blur: 10pt,
        spread: 1pt,
        fill: rgb(89, 85, 101, 50%),
        radius: 14pt,
        block(
          inset: 7mm,
          radius: 5mm,
          fill: white,
          ..args,
        ),
      ),
    ))

    // Positions
    let positions = (
      conditions: (x: 0, y: 0),
      spawn: (x: 6, y: 3.5),
      commands-success: (x: 11, y: 1),
      commands-fail: (x: 11, y: -1.5),
      conditions-met-no-command: (x: 15, y: 2.75),
      conditions-met-command-success: (x: 18.5, y: 2.75),
      conditions-met-command-fail: (x: 22, y: 2.75),
      conditions-not-met-no-command: (x: 15, y: -3.25),
      conditions-not-met-command-success: (x: 18.5, y: -3.25),
      conditions-not-met-command-fail: (x: 22, y: -3.25),
    )


    // Lines
    let line-start = 1.5

    anchor("spawn-start", (line-start, 1))
    anchor("spawn-end", (7.5, 1))
    line(
      "spawn-start",
      "spawn-end",
      (7.5, 4),
      stroke: (
        paint: gray.transparentize(60%),
        thickness: 3mm,
      ),
    )
    content(
      ("spawn-start", 50%, "spawn-end"),
      text(
        "No Conditions Set",
        font: "Nunito Sans 7pt",
        fill: gray.darken(50%),
        size: 7pt,
      ),
    )

    let y = 0
    let line-end = 10
    anchor("cmd-success-start", (line-start, y))
    anchor("cmd-success-end", (line-end, y))
    line(
      "cmd-success-start",
      "cmd-success-end",
      stroke: (
        paint: genome-colours.spawn.transparentize(40%),
        thickness: 3mm,
      ),
    )
    content(
      ("cmd-success-start", 50%, "cmd-success-end"),
      text(
        "Conditions Satisfied",
        font: "Nunito Sans 7pt",
        fill: gray.darken(50%),
        size: 7pt,
      ),
    )

    let commands-line-start = positions.commands-success.x + 1.2
    anchor("conditions-met-no-command-start", (
      commands-line-start,
      positions.commands-success.y,
    ))
    anchor("conditions-met-no-command-end", (
      positions.conditions-met-no-command.x,
      positions.commands-success.y,
    ))
    line(
      "conditions-met-no-command-start",
      "conditions-met-no-command-end",
      positions.conditions-met-no-command,
      stroke: (
        paint: gray.transparentize(60%),
        thickness: 3mm,
      ),
    )
    content(
      ("conditions-met-no-command-start", 50%, "conditions-met-no-command-end"),
      text(
        "No Commands Set",
        font: "Nunito Sans 7pt",
        fill: gray.darken(50%),
        size: 7pt,
      ),
    )

    line(
      (commands-line-start - 0.5, positions.commands-success.y - 0.5),
      (
        positions.conditions-met-command-success.x,
        positions.commands-success.y - 0.5,
      ),
      positions.conditions-met-command-success,
      stroke: (
        paint: genome-colours.spawn.transparentize(40%),
        thickness: 3mm,
      ),
    )

    line(
      (commands-line-start - 0.5, positions.commands-success.y - 1),
      (
        positions.conditions-met-command-fail.x,
        positions.commands-success.y - 1,
      ),
      positions.conditions-met-command-fail,
      stroke: (
        paint: genome-colours.command.transparentize(40%),
        thickness: 3mm,
      ),
    )

    let y = y - calc.abs(1 - y)
    anchor("cmd-fail-start", (line-start, y))
    anchor("cmd-fail-end", (line-end, y))
    line(
      "cmd-fail-start",
      "cmd-fail-end",
      stroke: (
        paint: genome-colours.command.transparentize(40%),
        thickness: 3mm,
      ),
    )
    content(
      ("cmd-fail-start", 50%, "cmd-fail-end"),
      text(
        "Conditions Not Satisfied",
        font: "Nunito Sans 7pt",
        fill: gray.darken(50%),
        size: 7pt,
      ),
    )

    line(
      (commands-line-start - 0.5, y),
      (
        positions.conditions-not-met-command-fail.x,
        y,
      ),
      positions.conditions-not-met-command-fail,
      stroke: (
        paint: genome-colours.command.transparentize(40%),
        thickness: 3mm,
      ),
    )

    line(
      (commands-line-start - 0.5, y - 0.5),
      (
        positions.conditions-not-met-command-success.x,
        y - 0.5,
      ),
      positions.conditions-not-met-command-success,
      stroke: (
        paint: genome-colours.spawn.transparentize(40%),
        thickness: 3mm,
      ),
    )

    anchor("conditions-not-met-no-command-start", (
      commands-line-start,
      positions.commands-fail.y - 0.5,
    ))
    anchor("conditions-not-met-no-command-end", (
      positions.conditions-not-met-no-command.x,
      positions.commands-fail.y - 0.5,
    ))
    line(
      "conditions-not-met-no-command-start",
      "conditions-not-met-no-command-end",
      positions.conditions-not-met-no-command,
      stroke: (
        paint: gray.transparentize(60%),
        thickness: 3mm,
      ),
    )
    content(
      (
        "conditions-not-met-no-command-start",
        50%,
        "conditions-not-met-no-command-end",
      ),
      text(
        "No Commands Set",
        font: "Nunito Sans 7pt",
        fill: gray.darken(50%),
        size: 7pt,
      ),
    )

    // Blocks
    content(
      positions.conditions,
      title-block(
        "Condition Check",
        stack(
          dir: ttb,
          spacing: 5mm,
          conditional-content((3, 4)),
          conditional-content((5, 6)),
        ),
      ),
    )

    content(
      positions.spawn,
      title-block(
        "Spawn Actions",
        spawn-content,
        inset: (x: 7mm, y: 5mm),
      ),
    )

    content(
      positions.commands-success,
      title-block(
        "Execute Commands",
        indexed-content((command(),), ("9/15",)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.commands-fail,
      title-block(
        "",
        indexed-content((command(),), ("12/18",)),
        inset: (x: 7mm, y: 4.5mm),
      ),
    )

    content(
      positions.conditions-met-no-command,
      title-block(
        "",
        indexed-content((fallback(),), (7,)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.conditions-met-command-success,
      title-block(
        "Change the Active Gene",
        indexed-content((fallback(),), ("10/16",)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.conditions-met-command-fail,
      title-block(
        "",
        indexed-content((fallback(),), ("11/17",)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.conditions-not-met-no-command,
      title-block(
        "",
        indexed-content((fallback(),), (8,)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.conditions-not-met-command-success,
      title-block(
        "",
        indexed-content((fallback(),), ("13/19",)),
        inset: (x: 7mm, y: 4mm),
      ),
    )

    content(
      positions.conditions-not-met-command-fail,
      title-block(
        "",
        indexed-content((fallback(),), ("14/20",)),
        inset: (x: 7mm, y: 4mm),
      ),
    )
  },
)
