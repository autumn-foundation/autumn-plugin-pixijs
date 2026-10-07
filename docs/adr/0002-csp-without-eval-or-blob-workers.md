# ADR 0002: Run PixiJS under `script-src 'self'` with no eval and no blob workers

- Status: accepted
- Date: 2026-10-07
- Applies to: autumn-plugin-pixijs 0.1.0

## Context

The default Autumn CSP is `default-src 'self'; img-src 'self' data:;
script-src 'self'; connect-src 'self'`. It has no `'unsafe-eval'` and no
`worker-src`, so workers fall back to `script-src 'self'`.

PixiJS 8 has two paths that this CSP blocks:

1. At renderer start, PixiJS tests `new Function(...)`. It also makes
   uniform and shader sync functions with `new Function`. A blocked
   `eval` sends a CSP violation report.
2. The texture loader prefers a worker that it starts from a `blob:` URL.

## Decision

- Load the upstream `unsafe-eval` package after PixiJS. It replaces the
  `eval` check and each generated function with plain functions.
- Before the first load, call
  `PIXI.Assets.setPreferences({ preferWorkers: false })`. Textures decode
  on the main thread with `createImageBitmap`.
- SVG sprites load through a `data:` image URL. `img-src 'self' data:`
  allows it.
- The E2E tests record `securitypolicyviolation` events in each page and
  fail on any event. They run with the default CSP and in nonce mode.

## Consequences

- Zero CSP violations with the default CSP and in nonce mode.
- The `unsafe-eval` functions are a little slower than generated
  functions. 2D scenes of typical size do not show a difference.
- Texture decode runs on the main thread.
- Images from other origins need `img-src` and `connect-src` entries.
