// E2E: the plugin works under Autumn's CSP nonce mode.
// In nonce mode `style-src` and `script-src` allow 'self' + nonce only.
import { after, before, test } from "node:test";
import assert from "node:assert/strict";

import { assertClean, pixel, start, waitState } from "./harness.mjs";

let app;
before(async () => {
  // autumn-web 0.8.0 documents AUTUMN_SECURITY__HEADERS__CSP_NONCE__ENABLED
  // but does not read it, so use autumn.toml.
  app = await start({ toml: "[security.headers.csp_nonce]\nenabled = true\n" });
});
after(async () => {
  await app?.close();
});

test("nonce mode: the CSP is strict and stages still render", async () => {
  const response = await fetch(`${app.base}/basic`);
  const csp = response.headers.get("content-security-policy");
  assert.match(csp, /script-src 'self' 'nonce-/);
  assert.doesNotMatch(csp, /unsafe-inline|unsafe-eval/);

  const page = await app.open("/basic");
  await waitState(page, "stage", "ready");
  const [r] = await pixel(page, "stage");
  assert.ok(r > 200, "red rectangle renders");
  await assertClean(page, assert);
});

test("nonce mode: every object kind, text, and the canvas renderer work", async () => {
  for (const path of ["/kinds", "/text", "/canvas", "/motion"]) {
    const page = await app.open(path);
    await waitState(page, "stage", "ready");
    await assertClean(page, assert);
  }
});

test("nonce mode: a tap sends the htmx request", async () => {
  const page = await app.open("/tap");
  await waitState(page, "stage", "ready");
  await page.waitForLoadState("load");
  const box = await page.locator("#stage > canvas").boundingBox();
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await page.waitForFunction(() => document.getElementById("log").textContent === "tapped 1");
  await assertClean(page, assert);
});
