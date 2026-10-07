# CLAUDE.md: autumn-plugin-pixijs

PixiJS plugin for Autumn (`autumn-web` 0.8). Style: ASD-STE100 for docs
and comments (short sentences, active voice, simple present).

## Layout

| Path | Role |
|---|---|
| `src/assets.rs` | `PIXI_ASSETS` bundle, `VENDORED` pins. |
| `src/plugin.rs` | `PixiPlugin` (installs the bundle). |
| `src/script.rs` | `pixi_script()`, `pixi_stylesheet()`. |
| `src/stage.rs` | Typed builder → `data-pixi-*` markup. |
| `assets/parse.js` | Pure attribute parsers. Node-tested. |
| `assets/init.js` | Runtime: build, loop, taps, free. E2E-tested. |
| `assets/pixi.css` | Default size, hidden declarations, fallback. |
| `assets/pixi.min.js`, `assets/unsafe-eval.min.js` | Vendored PixiJS 8.22.0. Do not edit. |
| `scripts/vendor.sh` | Re-vendor PixiJS. |
| `scripts/make_fixture_images.py` | Writes `static/img/*`. |
| `examples/pixi_demo.rs` | Demo app. |
| `examples/e2e_fixture.rs` | E2E routes, one per scenario. |
| `tests/js/` | `node --test` unit tests for `parse.js`. |
| `tests/e2e/` | Playwright + Chromium (SwiftShader) tests. |
| `docs/plan.md`, `docs/adr/` | Plan, acceptance criteria, decisions. |

## Rules

- Keep Rust and JS in sync. A new attribute needs: builder method,
  `ATTR` entry in `parse.js`, parser test, E2E test, README row. The test
  `every_emitted_attribute_and_value_is_known_to_the_runtime` checks names.
- The builder never emits non-finite numbers (proptest).
- No inline script, inline style, import map, or `eval` (CSP). E2E tests
  fail on any CSP violation.
- Vendored files change only through `scripts/vendor.sh`. Update
  `VENDORED` and `assets/manifest.json` in the same commit.
- The compiler embeds `init.js`. Rebuild the fixture before you run E2E
  tests.
- `init.js` must not count its own canvas insertion as a stage change. A
  rebuild loop then blocks the page `load` event.
- E2E predicates: the page CSP blocks `eval`, so `page.waitForFunction`
  with a source string fails. Use `until()` (Node-side polling).

## Commands

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings
cargo test
cargo llvm-cov --lib --fail-under-lines 85 --summary-only
npm ci && npm run test:unit
npx playwright install chromium   # one time
cargo build --example e2e_fixture --example pixi_demo && npm run test:e2e
E2E_SKIP_COVERAGE=1 node --test --test-name-pattern="<name>" tests/e2e/stage.test.mjs
```

## Known problems

- `pkill -f e2e_fixture` also matches your own shell command. Use
  `pkill -f "examples/[e]2e_fixture"`.
- autumn-web 0.8.0 does not read `AUTUMN_SECURITY__HEADERS__CSP_NONCE__ENABLED`.
  The nonce E2E test writes an `autumn.toml` (`AUTUMN_MANIFEST_DIR`).
- htmx injects an inline `<style>`. Pages set
  `includeIndicatorStyles: false` for nonce mode.
