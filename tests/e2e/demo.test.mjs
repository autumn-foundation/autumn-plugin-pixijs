// E2E smoke test of examples/pixi_demo.rs: every stage builds, the coin
// game and the gallery swap through htmx, the custom script runs, and the
// page is clean.
import { after, before, test } from "node:test";
import assert from "node:assert/strict";

import { assertClean, read, start, until } from "./harness.mjs";

let app;
before(async () => {
  app = await start({ example: "pixi_demo", ready: "/" });
});
after(async () => {
  await app?.close();
});

/** States of all stages on the page. */
const states = (page) =>
  page.evaluate(() => [...document.querySelectorAll('[data-pixi="stage"]')].map((e) => e.getAttribute("data-pixi-state")));

test("the demo page builds every stage with no errors", async () => {
  const page = await app.open("/", { viewport: { width: 1000, height: 900 } });
  await page.waitForFunction(
    () => [...document.querySelectorAll('[data-pixi="stage"]')].every((e) => e.getAttribute("data-pixi-state") === "ready"),
    null,
    { timeout: 20_000 },
  );
  assert.equal((await states(page)).length, 5);
  await page.waitForLoadState("load");

  // Coin game: a tap on a coin posts to the server, which sends the next stage.
  await page.locator("#game").scrollIntoViewIfNeeded();
  const coins = () => read(page, "game", (h) => h.objects.filter((o) => o.label.startsWith("coin-")).length);
  assert.equal(await coins(), 5);
  const point = await read(page, "game", (h) => {
    const coin = h.root.getChildByLabel("coin-0");
    const global = coin.getGlobalPosition();
    return { x: global.x, y: global.y };
  });
  const box = await page.locator("#game > canvas").boundingBox();
  await page.mouse.click(box.x + point.x, box.y + point.y);
  await until(page, "game", (h) => h.objects.filter((o) => o.label.startsWith("coin-")).length === 4);
  assert.equal(await read(page, "game", (h) => h.root.getChildByLabel("score").text), "Score: 1 / 5");

  // Reset: the server sends a full game.
  await page.click("text=Reset");
  await until(page, "game", (h) => h.objects.filter((o) => o.label.startsWith("coin-")).length === 5);

  // Gallery: a new stage replaces the old one, and the canvas count stays.
  const old = await page.evaluate(() => {
    window.__oldGallery = document.querySelector("#gallery [data-pixi]");
    return document.querySelectorAll("canvas").length;
  });
  await page.click("text=Next shape");
  await page.waitForFunction(() => {
    const stage = document.querySelector("#gallery [data-pixi]");
    return stage !== window.__oldGallery && stage?.getAttribute("data-pixi-state") === "ready";
  });
  assert.equal(await page.evaluate(() => window.__oldGallery.getAttribute("data-pixi-state")), "disposed");
  assert.equal(await page.evaluate(() => document.querySelectorAll("canvas").length), old);

  // static/js/demo.js: the ball moves.
  await page.locator("#custom").scrollIntoViewIfNeeded();
  const x = () => read(page, "custom", (h) => h.root.getChildByLabel("ball").x);
  const before = await x();
  await until(page, "custom", new Function(`return (h) => h.root.getChildByLabel("ball").x !== ${before}`)());

  await assertClean(page, assert);
});
