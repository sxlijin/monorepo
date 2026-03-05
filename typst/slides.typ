#set page(
  width: 16cm,
  height: 9cm,
  margin: 0.8cm,
)

#set text(size: 18pt)
#set par(justify: false)

#let slide(title, body) = [
  #align(center + top, text(30pt, weight: "bold")[#title])
  #v(0.6cm)
  #body
  #pagebreak()
]

#slide("Typst Slides", [
  #align(center + horizon, [
    A minimal, dependency-free slide deck example.
  ])
])

#slide("Agenda", [
  - Why Typst for slides
  - Basic layout and styles
  - Reusable slide helper
])

#slide("Code Sample", [
  - Define a reusable helper:
  - `#let slide(title, body) = [ ... ]`
  - Use it to build each page consistently.
])

#slide("Two Columns", [
  #grid(
    columns: (1fr, 1fr),
    gutter: 1cm,
    [
      *Left*
      - point A
      - point B
    ],
    [
      *Right*
      - point C
      - point D
    ],
  )
])

#slide("Thanks", [
  #align(center + horizon, text(26pt)[Questions?])
])
