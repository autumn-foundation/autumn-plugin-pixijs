# Plan: autumn-plugin-pixijs 0.1.0

Target: Autumn `autumn-web` 0.8.0. Prior art: `autumn-plugin-motion` 0.2.0
and `autumn-plugin-three` 0.1.0. Style: ASD-STE100. Method: SPEC → RED →
GREEN → REFACTOR.

## 1. Goal

Give Autumn apps 2D canvas scenes with PixiJS. Do not use npm, a bundler,
or inline script. Write the scene in Rust or in HTML attributes. Keep the
scenes correct across htmx swaps. Let a click on a canvas object start an
htmx request.

## 2. Acceptance criteria

No issue for this plugin exists (checked: this repository has no issues,
and no `autumn-foundation/autumn` issue names PixiJS). These criteria come
from the request and from the sibling plugins.

| ID | Criterion |
|---|---|
| AC1 | The crate targets `autumn-web` 0.8, Rust 1.88 (MSRV), and edition 2024. |
| AC2 | `PixiPlugin` serves vendored PixiJS 8.22.0 and the PixiJS `unsafe-eval` package through `PluginAssets`. A `sha384` pin locks each vendored file. The plugin passes the Autumn plugin conformance check. |
| AC3 | `pixi_script()` and `pixi_stylesheet()` emit tags with SRI. There is no inline script. Stages work with the default Autumn CSP and in nonce mode, with zero CSP violations. |
| AC4 | A typed Maud builder (`Stage`, `Sprite`, `Shape`, `Text`, `TilingSprite`, `AnimatedSprite`) renders `data-pixi-*` markup. Hand-written markup also works. The builder never emits a non-finite number. |
| AC5 | The runtime builds stages on load, after htmx swaps, and on DOM insertion. It builds a stage again when its declarations change. It frees the renderer when the stage leaves the document. It never builds a stage two times. |
| AC6 | A tappable object sends a bubbling `pixi:tap` DOM event from its declaration element. An htmx trigger can use it. Keyboard users can tap the object. |
| AC7 | Custom JavaScript gets `pixi:ready` and `pixi:error` events and an `element.autumnPixi` handle. The page has one `PIXI` global. |
| AC8 | A bad attribute value uses the default. A bad kind or URL skips the object and logs a warning. A failed image load shows the fallback, or keeps the other objects when there is no fallback. The fallback shows without JavaScript or a renderer. |
| AC9 | Reduced motion stops automatic motion unless the stage opts in. The loop stops off screen. A static stage renders only on change. The pixel ratio is at most 2. A label gives `role="img"`. |
| AC10 | `cargo fmt`, `cargo clippy` (pedantic + nursery, `-D warnings`), and all tests pass. Line coverage: Rust ≥ 85 %, `parse.js` ≥ 85 %, `init.js` (E2E) ≥ 95 %. CI runs all gates. |
| AC11 | README, CLAUDE.md, this plan, ADRs, and rustdoc exist. They use ASD-STE100. |
| AC12 | A demo app shows the features. An E2E test checks it. |

## 3. Brainstorming

Ideas (all ideas first, no filter):

1. Vendor PixiJS in the crate and serve it with `PluginAssets`.
2. Use the ES module build (`pixi.min.mjs`).
3. Use the IIFE build (`pixi.min.js`, global `PIXI`).
4. Vendor the `unsafe-eval` package so PixiJS needs no `eval`.
5. Typed Rust builder: `Stage`, `Sprite`, `Shape`, `Text`, tiling and
   animated sprites.
6. Hidden child declarations, one per display object (as in the three
   plugin).
7. One JSON attribute for the whole stage.
8. A logical stage size (for example `800 × 450`). The runtime scales it
   to the element. Server code uses stable coordinates.
9. `pixi:tap` DOM events from the declaration element. htmx uses them as
   triggers (`hx-trigger="pixi:tap"`).
10. Keyboard access for tappable objects through the PixiJS accessibility
    system.
11. Spin, tiling scroll, and sprite-sheet animation as declarative motion.
12. Reduced-motion guard and an off-screen pause.
13. Canvas 2D fallback renderer when WebGL is not available.
14. WebGPU renderer.
15. Filters, masks, particles, physics, sound.
16. Update one object in place when its declaration changes.
17. `pixi:ready` handle with the `Application` and the `PIXI` global.
18. Headless Chromium tests that read real canvas pixels.

Selected: 1, 3, 4, 5, 6, 8–13, 17, 18. Rejected:

- 2: the `unsafe-eval` package is an IIFE that patches the global `PIXI`.
  Community packages (filters, sound) also use the global. The ES module
  build has no global. See ADR 0001.
- 7: hard to write by hand; children are htmx-friendly.
- 14: no headless test path (SwiftShader has no WebGPU). Record as a limit.
- 15: large scope. Use `pixi:ready` for custom code. Record as a limit.
- 16: a rebuild is fast (the asset cache keeps textures) and simple.

## 4. Reverse brainstorming

Question: "How can we make this plugin fail?" Then invert each answer.

| Way to fail | Prevention |
|---|---|
| PixiJS calls `new Function`. The default CSP blocks it and reports a violation. | Load the upstream `unsafe-eval` package after PixiJS. It replaces each `eval` path. An E2E test checks for zero CSP violations. |
| The texture loader starts a worker from a `blob:` URL. `script-src 'self'` blocks it. | `Assets.setPreferences({ preferWorkers: false })` before any load. |
| Inline script or import map blocked by CSP. | External files only. Classic `defer` scripts and one module script, all with SRI. |
| Vendored bytes drift from upstream. | `sha384` pins in Rust, `vendor.sh`, and `manifest.json`. Tests compare them. |
| htmx swaps leak WebGL contexts (browser limit ≈16). | Destroy the application on `htmx:beforeCleanupElement` and when the element leaves the document. A test checks `isContextLost()`. |
| A stage builds two times. | A `Map` from element to state. |
| `NaN` or `Infinity` in attributes. | Rust: never emit non-finite numbers (proptest). JS: reject non-finite input. |
| A huge polygon or star from an attribute stops the page. | Clamp counts and sizes in `parse.js`. |
| A bad value throws and stops all stages. | Parse defensively. Catch per stage. Show the fallback. |
| An image fails to load. | `pixi:error` event. With a fallback: show it. Without: keep the other objects. |
| `app.init()` resolves after the element left the page. | Check `disposed` after each `await`. Destroy late objects. |
| `AnimatedSprite` uses the shared ticker and runs off screen. | `autoUpdate: false`. The stage loop updates it. |
| Text is blurry when the stage scales up. | Text resolution follows the stage scale. |
| Keyboard users cannot tap canvas objects. | PixiJS accessibility layer: tappable objects get a focusable button with the label. |
| Reduced-motion users see motion. | No automatic motion unless `data-pixi-reduced="animate"`. |
| `pixi_script()` is missing, or loads after the runtime. | The runtime checks the `PIXI` global. Without it: `pixi:error` and the fallback. |
| A page module adds its `pixi:ready` listener after the first scan. | The first scan waits for `DOMContentLoaded`. |
| Tests check strings only. | E2E tests in headless Chromium read canvas pixels. |

## 5. Six thinking hats

- **White (facts).** PixiJS 8.22.0 is the newest release (2026-10-01).
  `dist/pixi.min.js` is an IIFE that sets `var PIXI`.
  `dist/packages/unsafe-eval.min.js` is an IIFE that patches
  `PIXI.AbstractRenderer`, `UboSystem`, and the shader systems. PixiJS
  checks `new Function` at renderer start. The texture loader prefers
  `blob:` workers. `autoDetectRenderer` takes a preference list, and 8.22
  has a Canvas 2D renderer. Autumn 0.8 serves `PluginAssets` with hashed
  URLs and SRI. Default CSP: `script-src 'self'`, `img-src 'self' data:`,
  `connect-src 'self'`, no `worker-src`. We do not use Verus: the crate has
  no `unsafe` code and no Rust state machine. Proptests cover the builder
  invariants. E2E tests cover the JS lifecycle.
- **Red (feelings).** Users want "a game-like widget on my page in ten
  lines". A click on a coin must reach the server with no custom JS.
- **Black (risks).** 840 KB of minified JS per page (gzip ≈ 250 KB). The
  IIFE build defines one global. Each stage has its own WebGL context. The
  accessibility layer appends a DOM overlay. Texture cache entries stay
  for the page lifetime.
- **Yellow (benefits).** One crate, no build step. Typed API catches
  errors at compile time. Canvas objects become htmx triggers. SRI on all
  tags. A Canvas 2D fallback covers machines without WebGL.
- **Green (creative).** Declarations are DOM proxies for display objects:
  they carry `id`, `hx-*`, and receive `pixi:tap`. Logical stage size keeps
  server coordinates stable at all screen sizes.
- **Blue (process).** Write the spec (this file and the ADRs). Then RED
  tests per slice, GREEN code, REFACTOR. Slices: assets → plugin → script
  tags → builder → `parse.js` → `init.js` → example → E2E → docs → review.

## 6. Spec (invariants)

1. The bundle holds exactly the served files. `manifest.json` and the
   license are not served.
2. Each vendored file matches its pinned `sha384`. The files are not
   changed.
3. `pixi_script()` loads `pixi.min.js`, then `unsafe-eval.min.js`, then
   `init.js`, in this order, each with SRI. It preloads `parse.js` with SRI.
4. The builder never emits a non-finite number.
5. One stage element owns at most one renderer. Removal frees it.
6. Reduced motion: no automatic motion without opt-in.
7. A bad declaration never stops other stages.
8. Every attribute and keyword that the builder emits is known to
   `parse.js`.

## 7. Slices

| # | Slice | RED test first |
|---|---|---|
| 1 | Asset bundle + manifest | `assets.rs` tests |
| 2 | `PixiPlugin` | serve, 404, routes, conformance |
| 3 | `pixi_script()` / `pixi_stylesheet()` | tag tests |
| 4 | Builder | unit + proptest |
| 5 | `parse.js` | `node --test` |
| 6 | `init.js` runtime | E2E (Chromium + WebGL) |
| 7 | Example + fixture images | E2E |
| 8 | CI, README, ADRs, CLAUDE.md | review |

## 8. Review (REFACTOR)

Five review agents checked the GREEN code: runtime correctness, security
and CSP, Rust API, tests and CI, and documentation. Each finding got a test
first, then a fix. The main fixes:

| Area | Finding | Fix |
|---|---|---|
| Security | The `NUMBER` regex backtracks (ReDoS) on long digit runs. | Linear regex; numbers have at most 64 characters. |
| Security | The worker setting ran only at the first build. | `init.js` sets it when it loads. |
| Security | `ParticleContainer` still used `new Function`. | Patch `PIXI.ParticleBuffer`. |
| Security | `tap_post` fails with CSRF on. | Demo and README use the Autumn CSRF helper. E2E test with CSRF on. |
| Security | Large text and extreme sizes use too much memory. | Text texture budget, aspect clamp, object limit. |
| Runtime | Accessibility buttons were offset by the stage position. | Tag the layer; CSS removes the move. |
| Runtime | Stages blocked touch scrolling. | `touch-action: auto` or `manipulation`. |
| Runtime | htmx settle removed the aspect ratio. | Set it again on `htmx:afterSettle`. |
| Runtime | `data-pixi` removal leaked a context; kind removal did not rebuild. | Watch `data-pixi` and kind attributes. |
| Runtime | One batch could build a stage two times. | Skip the rebuild of a stage built in the batch. |
| Runtime | Star anchors used a wrong box. | Pivot from the shape bounds. |
| Runtime | A DPR change kept the old resolution. | `matchMedia` resolution listener. |
| Runtime | `auto` did not fall back when the WebGL renderer failed. | Retry with Canvas 2D. |
| Accessibility | `role="img"` hid the buttons. | `role="group"` with tappable objects. |
| Accessibility | Canvas 2D had no keyboard access. | Register the PixiJS accessibility system for Canvas 2D. |
| Rust | Two tap verbs; options without a verb. | One verb; options need it. |
| Rust | Limits and defaults were copied by hand. | Tests compare them with `parse.js`. |
| Tests | Weak assertions and fixed sleeps. | State-transition recorder, frame counters, per-test cleanup. |
| Docs | Wrong claims (GIF, state diagram, defaults). | Corrected; ASD-STE100 rewrites. |
