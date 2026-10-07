# ADR 0003: Declarative stages with child declarations and a freeing runtime

- Status: accepted
- Date: 2026-10-07
- Applies to: autumn-plugin-pixijs 0.1.0

## Context

Autumn renders HTML on the server. htmx swaps HTML fragments. A PixiJS
application is a JavaScript object graph. With WebGL, it holds a GPU
context. Browsers allow about 16 WebGL contexts per page. A swapped-out
stage that is not destroyed leaks its context.

Server code needs stable coordinates. The element size changes with the
screen.

## Decision

- A stage is one element: `<div data-pixi="stage">`. Stage options are
  attributes on it.
- Each display object is a hidden child declaration (`data-pixi-sprite`,
  `data-pixi-tiling`, `data-pixi-sheet`, `data-pixi-text`,
  `data-pixi-shape`). Later declarations draw on top. Text is the element
  content, so an htmx swap can change it.
- Coordinates use a logical stage size (default `800 × 450`). The runtime
  sets the element aspect ratio to this size (clamped to `1/10`–`10`) and
  scales one root container to fit the element. Objects center on their
  position by default. A shape anchor sets its pivot within its bounds.
- htmx settle puts back the old `style` of an element with the same id.
  The runtime sets the aspect ratio again on `htmx:afterSettle`.
- `parse.js` is pure and never throws. Node tests cover it. `init.js` turns
  the config into PixiJS objects.
- `init.js` scans on `DOMContentLoaded`, on `htmx:afterSwap`, and on DOM
  insertion (`MutationObserver`). A `Map` from element to state prevents a
  second build.
- The runtime builds a stage again when a declaration is added or removed,
  when a watched attribute or the text of a declaration changes, when a
  stage attribute changes, or when its canvas is removed. A stage that the
  same mutation batch built does not build again. The `data-pixi`
  attribute starts or stops a stage.
- A stage that failed stays failed (a `WeakSet` parks it). It builds again
  only when it leaves the document and comes back, or when its
  declarations change.
- Removal frees the stage: `app.destroy()` loses the WebGL context. The
  asset cache keeps textures for other stages.
- A tappable object gets `eventMode = "static"` and the PixiJS
  accessibility layer (a focusable button with the label). A tap sends a
  bubbling `pixi:tap` event from the declaration element. Thus an htmx
  attribute on the declaration (`hx-trigger="pixi:tap"`) sends a request.
- PixiJS puts its accessibility layer inside the stage and moves it by the
  canvas position in the viewport. The runtime tags the layer
  (`data-pixi-a11y`), and `pixi.css` removes the move. A labeled stage
  with tappable objects is `role="group"`, so screen readers keep the
  buttons. PixiJS gives the layer to WebGL and WebGPU only. The runtime
  also registers it for Canvas 2D.
- PixiJS sets `touch-action: none` on its canvas. The runtime sets `auto`,
  or `manipulation` for a stage with tappable objects, so that the page
  scrolls.
- The ticker runs only while the stage is visible, has motion, and motion
  is allowed. Otherwise frames render on demand. `AnimatedSprite` does not
  use the shared ticker.
- Renderer: WebGL, then Canvas 2D (`auto`). When the WebGL renderer fails
  to start, `auto` tries Canvas 2D. `canvas` uses no WebGL context.

```mermaid
sequenceDiagram
    participant S as Server (Maud)
    participant H as htmx
    participant R as init.js
    participant P as PixiJS
    participant U as User
    S->>H: HTML with data-pixi-*
    H->>R: htmx:afterSwap / DOM insert
    R->>R: readStage() (parse.js)
    R->>P: Application.init(), Assets.load(), display objects
    R-->>H: pixi:ready (bubbles)
    U->>P: click, touch, or Enter
    P->>R: pointertap
    R-->>H: pixi:tap on the declaration
    H->>S: hx-post / hx-get
    S->>H: next stage HTML
    H->>R: removal / insertion
    R->>P: app.destroy() (frees the context)
```

```mermaid
stateDiagram-v2
    [*] --> loading: scan (load, htmx swap, DOM insert)
    loading --> ready: built
    loading --> error: no renderer, an image failed, or the context was lost
    loading --> disposed: element removed, or htmx cleanup
    ready --> error: WebGL context lost
    ready --> disposed: element removed, or htmx cleanup
    ready --> loading: declarations, stage attributes, or canvas changed
    error --> loading: element inserted again, or declarations changed
    error --> disposed: element removed
    disposed --> loading: element inserted again, or declarations changed
```

## Consequences

- Stages work in any htmx swap with no extra code.
- Server-driven canvas: a tap can return the next stage.
- A change rebuilds the whole stage. The asset cache makes this fast, but
  custom objects from `pixi:ready` code must be added again (the new
  `pixi:ready` event fires).
- Keyboard users reach tappable objects with Tab.
