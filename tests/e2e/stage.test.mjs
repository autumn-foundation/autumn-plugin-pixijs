// Browser E2E tests: real Chromium, real WebGL (SwiftShader).
// Run: cargo build --example e2e_fixture && npm run test:e2e
import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";

import { assertClean as clean, lineCoverage, pixel, read, sleep, start, until, waitState } from "./harness.mjs";

/** Minimum line coverage of init.js over this suite. */
const MIN_INIT_COVERAGE = 95;

let app;
before(async () => {
  app = await start();
});
after(async () => {
  await app?.close();
  if (process.env.E2E_SKIP_COVERAGE) return; // Filtered dev runs.
  const init = lineCoverage("init");
  console.log(`init.js line coverage: ${init?.percent.toFixed(1)}% (uncovered lines: ${init?.uncovered.join(", ")})`);
  assert.ok(init && init.percent >= MIN_INIT_COVERAGE, `init.js coverage ${init?.percent} < ${MIN_INIT_COVERAGE}`);
});

const assertClean = (page) => clean(page, assert);
const events = (page) => page.evaluate(() => window.__events);
const eventTypes = async (page) => (await events(page)).map((e) => e[0]);

/** Init script: canvases cannot make a WebGL context. */
const NO_WEBGL = () => {
  const original = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (type, ...rest) {
    return /webgl/.test(type) ? null : original.call(this, type, ...rest);
  };
};

/** Inserts `html` at the end of the body. */
const insert = (page, html) => page.evaluate((h) => document.body.insertAdjacentHTML("beforeend", h), html);

describe("rendering", () => {
  test("a stage renders the declared shape over the background", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    const [r, g, b] = await pixel(page, "stage", 0.5, 0.5);
    assert.ok(r > 200 && g < 50 && b < 50, `center is red: ${[r, g, b]}`);
    const [r2, g2, b2] = await pixel(page, "stage", 0.02, 0.02);
    assert.ok(r2 < 50 && g2 < 50 && b2 > 200, `corner is the blue background: ${[r2, g2, b2]}`);
    assert.deepEqual(await events(page), [["pixi:ready", "stage", null]]);
    assert.deepEqual(await page.evaluate(() => window.__details), [true], "detail is the handle");
    assert.equal(await page.locator("#stage > canvas").count(), 1);
    assert.equal(await page.locator("#stage > canvas").getAttribute("aria-hidden"), "true");
    assert.equal(await page.locator("#stage").getAttribute("role"), "img");
    assert.equal(await page.locator(".fallback").isVisible(), false, "fallback hides when ready");
    await assertClean(page);
  });

  test("a page module after pixi_script() gets pixi:ready", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    assert.deepEqual(await page.evaluate(() => window.__lateReady), ["stage"]);
  });

  test("the handle exposes PixiJS for custom code", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    const info = await read(page, "stage", (h) => ({
      version: h.PIXI.VERSION,
      global: h.PIXI === window.PIXI,
      renderer: h.app.renderer.name,
      objects: h.objects.length,
      children: h.root.children.length,
      size: h.size,
      center: [h.objects[0].x, h.objects[0].y],
      label: h.objects[0].label,
    }));
    assert.deepEqual(info, {
      version: "8.22.0",
      global: true,
      renderer: "webgl",
      objects: 1,
      children: 1,
      size: [800, 450],
      center: [400, 225],
      label: "",
    });
  });

  test("PixiJS needs no eval and starts no blob workers", async () => {
    const page = await app.open("/kinds");
    await waitState(page, "stage", "ready");
    const info = await read(page, "stage", (h) => ({
      evalCheck: h.app.renderer._unsafeEvalCheck.toString().replace(/\s/g, ""),
      workers: h.PIXI.loadTextures.config.preferWorkers,
    }));
    assert.equal(info.evalCheck, "_unsafeEvalCheck(){}", "the unsafe-eval package is installed");
    assert.equal(info.workers, false);
    await assertClean(page);
  });

  test("hand-written markup works and bad declarations only warn", async () => {
    const page = await app.open("/handwritten");
    await waitState(page, "stage", "ready");
    const [r, g, b] = await pixel(page, "stage");
    assert.ok(g > 200 && r < 50 && b < 50, `green rectangle: ${[r, g, b]}`);
    const warnings = await page.evaluate(() => window.__warnings.join("\n"));
    for (const word of ["teapot", "javascript"]) assert.match(warnings, new RegExp(word));
    assert.equal(await read(page, "stage", (h) => h.objects[0].alpha), 1, "bad alpha uses the default");
    assert.equal(await read(page, "stage", (h) => h.app.renderer.name), "webgl", "bad renderer uses auto");
    await assertClean(page);
  });

  test("the stage size sets the aspect ratio and the scale", async () => {
    const page = await app.open("/size");
    await waitState(page, "custom", "ready");
    await waitState(page, "square", "ready");
    const square = await page.locator("#square").boundingBox();
    const custom = await page.locator("#custom").boundingBox();
    assert.ok(Math.abs(square.width - square.height) <= 1, JSON.stringify(square));
    assert.ok(Math.abs(custom.width / custom.height - 2) < 0.02, JSON.stringify(custom));
    const scale = await read(page, "custom", (h) => h.root.scale.x);
    assert.ok(Math.abs(scale - custom.width / 800) < 0.01, `scale ${scale}`);
  });

  test("the canvas follows the element size", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    const before = await read(page, "stage", (h) => h.app.canvas.width);
    await page.setViewportSize({ width: 400, height: 600 });
    await until(page, "stage", new Function(`return (h) => h.app.canvas.width !== ${before}`)());
    const { width, scale, screen } = await read(page, "stage", (h) => ({
      width: h.app.canvas.width,
      scale: h.root.scale.x,
      screen: h.app.screen.width,
    }));
    assert.ok(width < before, `${width} < ${before}`);
    assert.ok(Math.abs(scale - screen / 800) < 0.01, `root scale ${scale}`);
  });

  test("the resolution is capped at 2", async () => {
    const page = await app.open("/basic", { deviceScaleFactor: 3 });
    await waitState(page, "stage", "ready");
    assert.equal(await read(page, "stage", (h) => h.app.renderer.resolution), 2);
  });
});

describe("kinds", () => {
  test("every object kind builds", async () => {
    const page = await app.open("/kinds");
    await waitState(page, "stage", "ready");
    const info = await read(page, "stage", (h) => {
      const get = (label) => h.root.getChildByLabel(label);
      return {
        labels: h.objects.map((o) => o.label),
        sprite: [get("sprite").width, get("sprite").texture.width],
        svg: [get("svg").width, get("svg").height],
        tiling: [get("tiling").width, get("tiling").height, get("tiling") instanceof h.PIXI.TilingSprite],
        sheet: [get("sheet").totalFrames, get("sheet") instanceof h.PIXI.AnimatedSprite, get("sheet").autoUpdate],
        text: [get("text").text, get("text").style.align, get("text").style.wordWrap, get("text").style.wordWrapWidth],
        graphics: h.objects.filter((o) => o instanceof h.PIXI.Graphics).length,
        circle: Math.round(get("circle").getLocalBounds().width),
        ellipse: Math.round(get("ellipse").getLocalBounds().width),
        star: Math.round(get("star").getLocalBounds().width),
      };
    });
    assert.deepEqual(info.labels, [
      "sprite", "svg", "tiling", "sheet", "text", "rect", "rounded", "circle", "ellipse", "star", "polygon", "ring",
    ]);
    assert.deepEqual(info.sprite, [64, 64]);
    assert.deepEqual(info.svg, [32, 32]);
    assert.deepEqual(info.tiling, [200, 100, true]);
    assert.deepEqual(info.sheet, [2, true, false]);
    assert.deepEqual(info.text, ["Hello", "right", true, 200]);
    assert.equal(info.graphics, 7);
    assert.equal(info.circle, 40);
    assert.equal(info.ellipse, 60);
    assert.ok(info.star >= 45 && info.star <= 50, `star width ${info.star}`);
    await assertClean(page);
  });

  test("a sprite draws its image", async () => {
    const page = await app.open("/kinds");
    await waitState(page, "stage", "ready");
    // The red 64 × 64 sprite is at (100, 100) in an 800 × 450 stage.
    const [r, g, b] = await pixel(page, "stage", 100 / 800, 100 / 450);
    assert.ok(r > 200 && g < 50 && b < 50, `sprite pixel: ${[r, g, b]}`);
  });

  test("transform, style, and size values reach PixiJS", async () => {
    const page = await app.open("/params");
    await waitState(page, "stage", "ready");
    const info = await read(page, "stage", (h) => {
      const get = (label) => h.root.getChildByLabel(label);
      const rect = get("rect");
      const bounds = rect.getLocalBounds();
      const text = get("text").style;
      return {
        rect: [rect.x, rect.y, Math.round(rect.angle), rect.scale.x, rect.scale.y, rect.alpha, rect.tint],
        bounds: [bounds.x, bounds.y, bounds.width, bounds.height],
        sprite: [get("sprite").width, get("sprite").height, get("sprite").anchor.x, get("sprite").anchor.y],
        tiling: [get("tiling").width, get("tiling").height],
        text: [text.fontSize, text.fontFamily, text.fill, text.fontWeight, text.align, text.wordWrapWidth],
        star: Math.round(get("star").getLocalBounds().width),
      };
    });
    assert.deepEqual(info.rect, [50, 60, 90, 2, 3, 0.5, 0xff0000]);
    assert.deepEqual(info.bounds, [0, -50, 100, 50], "anchor 0,1 puts the rect above its position");
    assert.deepEqual(info.sprite, [20, 10, 1, 0]);
    assert.deepEqual(info.tiling, [400, 200], "a bad size uses the stage size");
    assert.deepEqual(info.text, [30, "monospace", 0x123456, "bold", "center", 120]);
    assert.ok(info.star >= 80 && info.star <= 90, `star with stroke: ${info.star}`);
  });
});

describe("motion", () => {
  /** Motion values of the /motion stage. */
  const motion = (page) =>
    read(page, "stage", (h) => {
      const get = (label) => h.root.getChildByLabel(label);
      return {
        looping: h.looping,
        rotation: get("spinner").rotation,
        tile: [get("tiles").tilePosition.x, get("tiles").tilePosition.y],
        frame: get("sheet").currentFrame,
      };
    });

  test("spin, scroll, and animations run", async () => {
    const page = await app.open("/motion");
    await waitState(page, "stage", "ready");
    await until(page, "stage", (h) => h.looping);
    const start = await motion(page);
    await sleep(400);
    const later = await motion(page);
    assert.ok(later.rotation > start.rotation, `spin: ${start.rotation} → ${later.rotation}`);
    assert.ok(later.tile[0] > start.tile[0] && later.tile[1] < start.tile[1], `scroll: ${start.tile} → ${later.tile}`);
    assert.notEqual(await read(page, "stage", (h) => h.root.getChildByLabel("sheet").playing), false);
    await assertClean(page);
  });

  test("reduced motion stops automatic motion", async () => {
    const page = await app.open("/motion", { reducedMotion: "reduce" });
    await waitState(page, "stage", "ready");
    const start = await motion(page);
    await sleep(300);
    const later = await motion(page);
    assert.equal(later.looping, false);
    assert.equal(later.rotation, start.rotation);
    assert.deepEqual(later.tile, start.tile);
    assert.equal(later.frame, start.frame);
  });

  test("a stage can opt back into motion", async () => {
    const page = await app.open("/motion-animate", { reducedMotion: "reduce" });
    await waitState(page, "stage", "ready");
    await until(page, "stage", (h) => h.looping);
  });

  test("a reduced-motion change at runtime stops the loop", async () => {
    const page = await app.open("/motion");
    await waitState(page, "stage", "ready");
    await until(page, "stage", (h) => h.looping);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await until(page, "stage", (h) => !h.looping);
    await page.emulateMedia({ reducedMotion: "no-preference" });
    await until(page, "stage", (h) => h.looping);
  });

  test("off-screen and static stages do not loop", async () => {
    const page = await app.open("/offscreen");
    await waitState(page, "stage", "ready");
    await sleep(200);
    assert.equal(await read(page, "stage", (h) => h.looping), false, "off screen");
    await page.locator("#stage").scrollIntoViewIfNeeded();
    await until(page, "stage", (h) => h.looping);
    const still = await app.open("/still");
    await waitState(still, "stage", "ready");
    await sleep(200);
    assert.equal(await read(still, "stage", (h) => h.looping), false, "no motion");
  });

  test("custom motion runs through the handle", async () => {
    const page = await app.open("/still");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      const h = document.getElementById("stage").autumnPixi;
      window.__ticks = 0;
      h.motion.push((dt) => {
        window.__ticks += dt > 0 ? 1 : 0;
      });
      h.update();
    });
    await until(page, "stage", (h) => h.looping);
    await page.waitForFunction(() => window.__ticks > 2);
  });

  test("requestRender() draws custom changes", async () => {
    const page = await app.open("/still");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      const h = document.getElementById("stage").autumnPixi;
      h.objects[0].tint = 0x00ff00;
      h.requestRender();
      h.requestRender(); // A second request in the same frame is a no-op.
    });
    await sleep(100);
    const [r, g] = await page.evaluate(() => {
      const { app } = document.getElementById("stage").autumnPixi;
      const gl = app.renderer.gl;
      const out = new Uint8Array(4);
      gl.readPixels(app.canvas.width / 2, app.canvas.height / 2, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, out);
      return [...out];
    });
    assert.ok(g > 200 && r < 50, `tinted green: ${[r, g]}`);
  });
});

describe("tap", () => {
  test("a click on a tappable object sends pixi:tap and the htmx request", async () => {
    const page = await app.open("/tap");
    await waitState(page, "stage", "ready");
    await page.waitForLoadState("load"); // htmx is a deferred script.
    const box = await page.locator("#stage > canvas").boundingBox();
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await page.waitForFunction(() => document.getElementById("log").textContent === "tapped 1");
    assert.deepEqual(await page.evaluate(() => window.__taps), [{ target: "button", id: "button", x: 400, y: 225 }]);
    const info = await read(page, "stage", (h) => {
      const button = h.root.getChildByLabel("button");
      return [button.eventMode, button.cursor, button.accessible, button.accessibleHint];
    });
    assert.deepEqual(info, ["static", "pointer", true, "Press me"]);
    await assertClean(page);
  });

  test("a tappable object without a request only sends pixi:tap", async () => {
    const page = await app.open("/tap");
    await waitState(page, "stage", "ready");
    const box = await page.locator("#stage > canvas").boundingBox();
    const scale = box.width / 800;
    await page.mouse.click(box.x + 60 * scale, box.y + 60 * scale);
    await page.mouse.click(box.x + 780 * scale, box.y + 430 * scale); // Empty corner.
    await sleep(200);
    const taps = await page.evaluate(() => window.__taps);
    assert.equal(taps.length, 1);
    assert.equal(taps[0].id, "plain");
    assert.equal(await page.locator("#log").textContent(), "none");
  });

  test("keyboard users can tap through the accessibility layer", async () => {
    const page = await app.open("/tap");
    await waitState(page, "stage", "ready");
    await page.waitForLoadState("load");
    await page.keyboard.press("Tab");
    const button = page.locator('button[aria-label="Press me"]');
    await button.waitFor({ state: "attached" });
    assert.equal(await page.locator('button[aria-label="plain"]').count(), 1, "the id names an unlabeled object");
    await button.focus();
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => document.getElementById("log").textContent === "tapped 1");
    const taps = await page.evaluate(() => window.__taps);
    assert.deepEqual(taps, [{ target: "button", id: "button", x: 400, y: 225 }]);
    await assertClean(page);
  });
});

describe("assets", () => {
  test("a missing image shows the fallback and fires pixi:error", async () => {
    const page = await app.open("/sprite-missing");
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.equal(await page.locator("#stage > canvas").count(), 0);
    const all = await events(page);
    assert.equal(all.length, 1);
    assert.equal(all[0][0], "pixi:error");
    assert.match(all[0][2], /missing\.png$/);
    assert.deepEqual(await page.evaluate(() => window.__details), [true], "detail.error is an Error");
    page.errors.length = 0; // The 404 logs an error. This is correct.
    await assertClean(page);
  });

  test("a missing image without a fallback keeps the other objects", async () => {
    const page = await app.open("/sprite-missing-no-fallback");
    await waitState(page, "stage", "error");
    assert.equal(await page.locator("#stage > canvas").count(), 1);
    const [r] = await pixel(page, "stage");
    assert.ok(r > 200, "the red rectangle stays");
    assert.equal(await read(page, "stage", (h) => h.objects.length), 1);
  });

  test("an image that does not decode fails the same way", async () => {
    const page = await app.open("/sprite-broken");
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
  });

  test("an unknown animation name warns and plays the first animation", async () => {
    const page = await app.open("/sheet-bad-animation");
    await waitState(page, "stage", "ready");
    assert.match(await page.evaluate(() => window.__warnings.join("\n")), /nope/);
    assert.equal(await read(page, "stage", (h) => h.root.getChildByLabel("sheet").totalFrames), 2);
  });

  test("a failed image is loaded again by the next build", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.route("**/static/img/red.png", (route) => route.abort());
    const html = (id) => `<div id="${id}" data-pixi="stage"><div hidden data-pixi-sprite="/static/img/red.png"></div></div>`;
    await insert(page, html("first"));
    await waitState(page, "first", "error");
    await page.unroute("**/static/img/red.png");
    await insert(page, html("second"));
    await waitState(page, "second", "ready");
    page.errors.length = 0; // The aborted request logs an error. This is correct.
  });
});

describe("progressive enhancement", () => {
  test("no WebGL: the auto renderer falls back to Canvas 2D", async () => {
    const page = await app.open("/basic", { init: NO_WEBGL });
    await waitState(page, "stage", "ready");
    assert.equal(await read(page, "stage", (h) => h.app.renderer.name), "canvas");
    const [r, g, b] = await pixel(page, "stage");
    assert.ok(r > 200 && g < 50 && b < 50, `center is red: ${[r, g, b]}`);
  });

  test("no WebGL with renderer webgl: the fallback shows and pixi:error fires", async () => {
    const page = await app.open("/webgl-only", { init: NO_WEBGL });
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.equal(await page.locator("#stage > canvas").count(), 0);
    assert.deepEqual(await eventTypes(page), ["pixi:error"]);
  });

  test("the canvas renderer draws without a WebGL context", async () => {
    const page = await app.open("/canvas");
    await waitState(page, "stage", "ready");
    assert.equal(await read(page, "stage", (h) => h.app.renderer.name), "canvas");
    const [r, g, b] = await pixel(page, "stage");
    assert.ok(r > 200 && g < 50 && b < 50, `center is red: ${[r, g, b]}`);
    const [r2, g2, b2] = await pixel(page, "stage", 0.02, 0.02);
    assert.ok(r2 < 50 && g2 < 50 && b2 > 200, `corner is blue: ${[r2, g2, b2]}`);
    await assertClean(page);
  });

  test("an unexpected build error shows the fallback", async () => {
    const page = await app.open("/basic", {
      init: () => {
        window.ResizeObserver = class {
          constructor() {
            throw new Error("boom");
          }
        };
      },
    });
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.equal(await page.locator("#stage > canvas").count(), 0);
    assert.deepEqual(await eventTypes(page), ["pixi:error"]);
  });

  test("PixiJS not loaded: the fallback shows and pixi:error names pixi_script()", async () => {
    const page = await app.open("/basic", {
      init: () => {
        Object.defineProperty(window, "PIXI", { get: () => undefined, set: () => {} });
      },
    });
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.match(await page.evaluate(() => window.__warnings.join("\n")), /pixi_script/);
  });

  test("no JavaScript: the fallback shows and declarations stay hidden", async () => {
    const page = await app.open("/basic", { javaScriptEnabled: false });
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.equal(await page.locator("[data-pixi-shape]").isVisible(), false);
    const box = await page.locator("#stage").boundingBox();
    assert.ok(Math.abs(box.width / box.height - 16 / 9) < 0.02, `default 16/9 box: ${JSON.stringify(box)}`);
  });
});

describe("lifecycle", () => {
  test("an htmx swap builds the new stage and frees the old one", async () => {
    const page = await app.open("/swap");
    await waitState(page, "first", "ready");
    await page.waitForLoadState("load");
    await page.evaluate(() => {
      window.__old = document.getElementById("first");
      window.__oldGl = window.__old.autumnPixi.app.renderer.gl;
    });
    await page.click("#next");
    await waitState(page, "second", "ready");
    assert.equal(await page.evaluate(() => window.__old.getAttribute("data-pixi-state")), "disposed");
    assert.equal(await page.evaluate(() => window.__oldGl.isContextLost()), true, "the old WebGL context is freed");
    assert.equal(await page.evaluate(() => window.__old.autumnPixi), undefined);
    assert.equal(await page.locator("canvas").count(), 1);
    const [, g] = await pixel(page, "second");
    assert.ok(g > 200, "the new stage renders");
    await assertClean(page);
  });

  test("removal without htmx frees the stage; a move does not", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      const el = document.getElementById("stage");
      window.__app = el.autumnPixi.app;
      const box = document.createElement("section");
      document.body.append(box);
      box.append(el); // A move: remove and insert in one task.
    });
    await sleep(100);
    assert.equal(await read(page, "stage", (h) => h.app === window.__app), true, "the move keeps the stage");
    await page.evaluate(() => {
      window.__gl = window.__app.renderer.gl;
      window.__el = document.getElementById("stage");
      window.__el.parentElement.remove();
    });
    await page.waitForFunction(() => window.__el.getAttribute("data-pixi-state") === "disposed");
    assert.equal(await page.evaluate(() => window.__gl.isContextLost()), true);
  });

  test("scans never build a stage twice", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      document.body.dispatchEvent(new CustomEvent("htmx:afterSwap", { bubbles: true, detail: { target: document.body } }));
      document.body.dispatchEvent(new CustomEvent("htmx:afterSwap", { bubbles: true, detail: {} }));
      document.body.appendChild(document.createElement("div"));
      document.body.appendChild(document.createTextNode("text"));
    });
    await sleep(200);
    assert.equal(await page.locator("#stage > canvas").count(), 1);
    assert.equal((await events(page)).length, 1);
  });

  test("init.js added after page load still scans the page", async () => {
    const page = await app.open("/late-script");
    await page.waitForLoadState("load");
    await page.evaluate(() => {
      const script = document.createElement("script");
      script.type = "module";
      script.src = document.body.dataset.init;
      document.head.append(script);
    });
    await waitState(page, "stage", "ready");
  });

  test("restored markup with a stale canvas builds again", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      const html = document.getElementById("stage").outerHTML.replace('id="stage"', 'id="restored"');
      document.body.insertAdjacentHTML("beforeend", html);
    });
    await waitState(page, "restored", "ready");
    await page.waitForFunction(() => window.__events.length === 2);
    assert.equal(await page.locator("#restored > canvas").count(), 1);
    const [r] = await pixel(page, "restored");
    assert.ok(r > 200, "the restored stage renders");
  });

  test("htmx cleanup frees a stage and parks it", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      const el = document.getElementById("stage");
      el.dispatchEvent(new CustomEvent("htmx:beforeCleanupElement", { bubbles: true }));
      document.body.dispatchEvent(new CustomEvent("htmx:beforeCleanupElement", { bubbles: true }));
      document.body.dispatchEvent(new CustomEvent("htmx:afterSwap", { bubbles: true, detail: { target: document.body } }));
    });
    await waitState(page, "stage", "disposed");
    await sleep(100);
    assert.equal(await page.locator("#stage").getAttribute("data-pixi-state"), "disposed", "a parked stage stays");
  });
});

describe("changes", () => {
  test("new children rebuild the stage", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      document.getElementById("stage").insertAdjacentHTML("afterbegin", '<div hidden data-pixi-shape="circle"></div>');
    });
    await page.waitForFunction(() => window.__events.length === 2);
    await waitState(page, "stage", "ready");
    assert.equal(await read(page, "stage", (h) => h.objects.length), 2);
    assert.equal(await page.locator("#stage > canvas").count(), 1);
  });

  test("text and attribute changes rebuild the stage", async () => {
    const page = await app.open("/text");
    await waitState(page, "stage", "ready");
    const count = () => page.evaluate(() => window.__events.length);
    await page.evaluate(() => {
      document.getElementById("score").textContent = "Score: 2";
    });
    await page.waitForFunction(() => window.__events.length === 2);
    assert.equal(await read(page, "stage", (h) => h.objects[0].text), "Score: 2");
    await page.evaluate(() => document.getElementById("score").firstChild.appendData("0"));
    await page.waitForFunction(() => window.__events.length === 3);
    assert.equal(await read(page, "stage", (h) => h.objects[0].text), "Score: 20");
    await page.evaluate(() => document.getElementById("score").setAttribute("data-pixi-fill", "#ff0000"));
    await page.waitForFunction(() => window.__events.length === 4);
    assert.equal(await read(page, "stage", (h) => h.objects[0].style.fill), 0xff0000);
    await page.evaluate(() => document.getElementById("stage").setAttribute("data-pixi-size", "400,400"));
    await page.waitForFunction(() => window.__events.length === 5);
    assert.deepEqual(await read(page, "stage", (h) => h.size), [400, 400]);
    // Changes that do not affect the stage do not rebuild it.
    await page.evaluate(() => {
      const el = document.getElementById("stage");
      el.setAttribute("class", "x");
      el.insertAdjacentHTML("beforeend", "<p>note</p>");
      el.lastElementChild.textContent = "changed";
      document.getElementById("score").setAttribute("title", "x");
    });
    await sleep(200);
    assert.equal(await count(), 5);
  });

  test("a failed stage stays failed when other content swaps in", async () => {
    const page = await app.open("/sprite-missing");
    await waitState(page, "stage", "error");
    await page.evaluate(() => {
      document.body.dispatchEvent(new CustomEvent("htmx:afterSwap", { bubbles: true, detail: { target: document.body } }));
      document.body.append(document.createElement("div"));
    });
    await sleep(200);
    assert.equal((await events(page)).length, 1);
    assert.equal(await page.locator("#stage").getAttribute("data-pixi-state"), "error");
    page.errors.length = 0;
  });

  test("a lost WebGL context shows the fallback", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => {
      document.getElementById("stage").autumnPixi.app.renderer.gl.getExtension("WEBGL_lose_context").loseContext();
    });
    await waitState(page, "stage", "error");
    assert.equal(await page.locator(".fallback").isVisible(), true);
    assert.equal(await page.locator("#stage > canvas").count(), 0);
    assert.deepEqual(await eventTypes(page), ["pixi:ready", "pixi:error"]);
  });

  test("a removed canvas rebuilds the stage", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    await page.evaluate(() => document.querySelector("#stage > canvas").remove());
    await page.waitForFunction(() => window.__events.length === 2);
    assert.equal(await page.locator("#stage > canvas").count(), 1);
  });
});

describe("removal during a build", () => {
  test("removal while an image loads frees the stage", async () => {
    const page = await app.open("/basic");
    await waitState(page, "stage", "ready");
    let release;
    let requested;
    const gate = new Promise((r) => (release = r));
    const inFlight = new Promise((r) => (requested = r));
    await page.route("**/static/img/checker.png", async (route) => {
      requested();
      await gate;
      await route.continue();
    });
    await insert(page, '<div id="late" data-pixi="stage"><div hidden data-pixi-sprite="/static/img/checker.png"></div></div>');
    await inFlight;
    await page.evaluate(() => {
      window.__late = document.getElementById("late");
      window.__late.remove();
    });
    release();
    await page.waitForFunction(() => window.__late.getAttribute("data-pixi-state") === "disposed");
    await sleep(300);
    assert.equal(await page.evaluate(() => window.__late.autumnPixi), undefined);
    assert.equal(await page.locator("canvas").count(), 1, "only the first stage has a canvas");
    await assertClean(page);
  });

  test("removal while the renderer starts frees the stage", async () => {
    const page = await app.open("/basic", {
      init: () => {
        let value;
        Object.defineProperty(window, "PIXI", {
          configurable: true,
          get: () => value,
          set: (v) => {
            if (v?.Application && !v.__slowInit) {
              v.__slowInit = true;
              const init = v.Application.prototype.init;
              v.Application.prototype.init = async function (...args) {
                await new Promise((r) => setTimeout(r, 300));
                return init.apply(this, args);
              };
            }
            value = v;
          },
        });
      },
    });
    await page.evaluate(() => {
      window.__el = document.getElementById("stage");
      window.__el.remove();
    });
    await sleep(600);
    assert.equal(await page.evaluate(() => window.__el.getAttribute("data-pixi-state")), "disposed");
    assert.equal(await page.locator("canvas").count(), 0);
    assert.deepEqual(await events(page), []);
    await assertClean(page);
  });
});
