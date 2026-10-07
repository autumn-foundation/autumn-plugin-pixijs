// autumn-plugin-pixijs runtime.
//
// Finds [data-pixi="stage"] elements and builds a PixiJS application for
// each. Scans on load, on htmx:afterSwap, and on any DOM insertion. Builds
// a stage again when its declarations change. Frees a stage when its
// element leaves the document. One element owns at most one renderer.
//
// PixiJS is the global `PIXI` (pixi.min.js). unsafe-eval.min.js patches it
// before this module runs, so PixiJS needs no `eval`.

import { ATTR, STAGE, readStage } from "./parse.js";

const SELECTOR = `[${ATTR.stage}="${STAGE}"]`;
/** Attributes that mark a declaration and its kind. */
const KINDS = [ATTR.sprite, ATTR.tiling, ATTR.sheet, ATTR.text, ATTR.shape];
const DECLARATION = KINDS.map((attr) => `[${attr}]`).join(",");
/** Stage attributes that change the build. */
const STAGE_ATTRS = new Set([ATTR.size, ATTR.background, ATTR.renderer, ATTR.reduced]);
/** Attributes that the observer watches. The runtime writes none of them. */
const WATCHED = ["id", ...Object.values(ATTR).filter((a) => ![ATTR.stage, ATTR.state, ATTR.canvas, ATTR.fallback].includes(a))];
/** Renderer preference lists per `data-pixi-renderer` value. */
const PREFERENCE = { auto: ["webgl", "canvas"], webgl: ["webgl"], canvas: ["canvas"] };
const DEG = Math.PI / 180;
const MAX_RESOLUTION = 2;
const MAX_TEXT_RESOLUTION = 4;
const MAX_DELTA_SECONDS = 0.1;
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

/** Live stages: element → state. */
const live = new Map();

/**
 * Elements that failed, or were freed by htmx, while in the document.
 * Scans skip them. They build again only after they leave the document
 * and come back, or when their declarations change.
 */
const parked = new WeakSet();

/** PixiJS settings for all stages. They run once. */
let configured = false;
function configure(PIXI) {
  if (configured) return;
  configured = true;
  // The texture loader starts workers from `blob:` URLs. `script-src 'self'` blocks them.
  PIXI.Assets.setPreferences({ preferWorkers: false });
}

/** Sends a bubbling event from `el`. */
function emit(el, type, detail) {
  el.dispatchEvent(new CustomEvent(type, { bubbles: true, detail }));
}

/** Marks `el` as failed: the fallback shows, `pixi:error` fires. */
function fail(el, error, src) {
  console.warn("autumn-plugin-pixijs:", src ?? "", error);
  parked.add(el);
  el.setAttribute(ATTR.state, "error");
  emit(el, "pixi:error", { error, src });
}

/** True when automatic motion is allowed for this stage. */
function motionAllowed(state) {
  return state.config.reducedAnimate || !reducedMotion.matches;
}

/** Renders one frame now. */
function renderNow(state) {
  if (state.started && !state.disposed) state.app.render();
}

/** Renders one frame on the next animation frame, unless the loop runs. */
function requestRender(state) {
  if (state.looping || state.frame || state.disposed) return;
  state.frame = requestAnimationFrame(() => {
    state.frame = 0;
    renderNow(state);
  });
}

/** Runs the loop only while the stage is ready, visible, and has allowed motion. */
function updateLoop(state) {
  const run = !state.disposed && state.ready && state.visible && state.motion.length > 0 && motionAllowed(state);
  if (run === state.looping) return;
  state.looping = run;
  if (run) {
    state.app.ticker.start();
  } else {
    state.app.ticker.stop();
    requestRender(state);
  }
}

/** Matches the renderer to the element size and scales the logical stage into it. */
function resize(state) {
  const { app, root, el } = state;
  const [width, height] = state.config.size;
  const resolution = Math.min(window.devicePixelRatio || 1, MAX_RESOLUTION);
  const w = Math.max(1, Math.round(el.clientWidth));
  const h = Math.max(1, Math.round(el.clientHeight));
  app.renderer.resize(w, h, resolution);
  const scale = Math.min(w / width, h / height);
  root.scale.set(scale);
  root.position.set((w - width * scale) / 2, (h - height * scale) / 2);
  // Text is a bitmap. Keep it sharp when the stage scales up.
  const textResolution = Math.min(MAX_TEXT_RESOLUTION, Math.max(1, resolution * scale));
  for (const text of state.texts) text.resolution = textResolution;
  renderNow(state);
}

/** Stops and frees a stage. The element shows its fallback again. */
function dispose(state) {
  if (state.disposed) return;
  state.disposed = true;
  live.delete(state.el);
  cancelAnimationFrame(state.frame);
  for (const observer of state.observers) observer.disconnect();
  if (state.started) {
    // Textures stay in the PixiJS asset cache. Other stages can use them.
    state.app.destroy({ removeView: true }, { children: true });
  }
  state.canvas?.remove();
  delete state.el.autumnPixi;
  state.el.setAttribute(ATTR.state, "disposed");
}

/** Loads the texture or sprite sheet of an object. Text and shapes need none. */
async function load(PIXI, o) {
  if (o.type === "text" || o.type === "shape") return null;
  const asset = await PIXI.Assets.load(o.src);
  const ok = o.type === "sheet" ? asset?.animations : asset instanceof PIXI.Texture;
  if (!ok) throw new Error(`not ${o.type === "sheet" ? "a sprite sheet" : "an image"}: ${o.src}`);
  return asset;
}

/** Draws a shape. The anchor moves the bounding box; it does not apply to polygons. */
function makeShape(PIXI, o) {
  const g = new PIXI.Graphics();
  const [ax, ay] = o.anchor;
  const a = o.args;
  switch (o.kind) {
    case "rect": g.rect(-ax * a[0], -ay * a[1], a[0], a[1]); break;
    case "rounded-rect": g.roundRect(-ax * a[0], -ay * a[1], a[0], a[1], a[2]); break;
    case "circle": g.circle((1 - 2 * ax) * a[0], (1 - 2 * ay) * a[0], a[0]); break;
    case "ellipse": g.ellipse((1 - 2 * ax) * a[0], (1 - 2 * ay) * a[1], a[0], a[1]); break;
    case "star": g.star((1 - 2 * ax) * a[1], (1 - 2 * ay) * a[1], a[0], a[1], a[2]); break;
    default: g.poly(a);
  }
  if (o.fill !== null) g.fill(o.fill);
  if (o.stroke !== null) g.stroke({ width: o.strokeWidth, color: o.stroke });
  return g;
}

/** Makes an animated sprite. An unknown animation name uses the first animation. */
function makeSheet(PIXI, state, o, sheet) {
  const names = Object.keys(sheet.animations);
  if (names.length === 0) throw new Error(`no animations in ${o.src}`);
  let name = o.animation ?? names[0];
  if (!names.includes(name)) {
    console.warn(`autumn-plugin-pixijs: no animation "${name}" in ${o.src}`);
    name = names[0];
  }
  const sprite = new PIXI.AnimatedSprite({ textures: sheet.animations[name], autoUpdate: false });
  sprite.animationSpeed = o.fps / 60;
  sprite.play();
  if (sprite.totalFrames > 1) {
    state.motion.push((dt) => sprite.update({ deltaTime: dt * 60, deltaMS: dt * 1000 }));
  }
  return sprite;
}

/** Makes the display object of a declaration. */
function make(PIXI, state, o, asset) {
  switch (o.type) {
    case "shape":
      return makeShape(PIXI, o);
    case "text": {
      const text = new PIXI.Text({
        text: o.text,
        style: {
          fontFamily: o.fontFamily,
          fontSize: o.fontSize,
          fill: o.fill,
          fontWeight: o.weight,
          align: o.align,
          wordWrap: o.wrap !== null,
          wordWrapWidth: o.wrap ?? 100,
        },
      });
      state.texts.push(text);
      return text;
    }
    case "tiling": {
      const [width, height] = o.size ?? state.config.size;
      const tiling = new PIXI.TilingSprite({ texture: asset, width, height });
      const [sx, sy] = o.scroll;
      if (sx !== 0 || sy !== 0) {
        state.motion.push((dt) => {
          tiling.tilePosition.x += sx * dt;
          tiling.tilePosition.y += sy * dt;
        });
      }
      return tiling;
    }
    case "sheet":
      return makeSheet(PIXI, state, o, asset);
    default: {
      const sprite = new PIXI.Sprite(asset);
      if (o.size) sprite.setSize(o.size[0], o.size[1]);
      return sprite;
    }
  }
}

/** Makes `object` tappable: pointer, keyboard (accessibility layer), and `pixi:tap`. */
function makeTappable(state, object, o) {
  object.eventMode = "static";
  object.cursor = "pointer";
  object.accessible = true;
  object.accessibleType = "button";
  object.accessibleTitle = object.accessibleHint = o.label ?? o.id ?? "button";
  object.on("pointertap", (event) => {
    // A keyboard tap from the accessibility layer has no pointer position.
    const point = event.nativeEvent ? state.root.toLocal(event.global) : object.position;
    emit(o.source, "pixi:tap", { id: o.id, x: point.x, y: point.y, object });
  });
  state.tappable = true;
}

/** Applies the shared settings and adds the object to the stage. */
function place(state, object, o) {
  const [width, height] = state.config.size;
  object.label = o.id ?? "";
  object.anchor?.set(o.anchor[0], o.anchor[1]);
  const [x, y] = o.position ?? [width / 2, height / 2];
  object.position.set(x, y);
  object.rotation = o.rotation * DEG;
  object.scale.set(object.scale.x * o.scale[0], object.scale.y * o.scale[1]);
  object.alpha = o.alpha;
  if (o.tint !== null) object.tint = o.tint;
  if (o.spin !== 0) state.motion.push((dt) => (object.rotation += o.spin * DEG * dt));
  if (o.tap) makeTappable(state, object, o);
  state.root.addChild(object);
  state.objects.push(object);
}

/** Starts the build of `el`. The build owns its errors. */
function build(el) {
  const config = readStage(el, document.baseURI);
  for (const warning of config.warnings) console.warn(`autumn-plugin-pixijs: ${warning}`);
  const state = {
    el,
    config,
    app: null,
    root: null,
    canvas: null,
    objects: [],
    texts: [],
    motion: [],
    observers: [],
    started: false,
    ready: false,
    visible: false,
    looping: false,
    tappable: false,
    disposed: false,
    frame: 0,
  };
  live.set(el, state);
  el.setAttribute(ATTR.state, "loading");
  el.style.aspectRatio = `${config.size[0]} / ${config.size[1]}`;
  // An htmx history snapshot can restore an old canvas. Remove it.
  for (const stale of el.querySelectorAll(`:scope > canvas[${ATTR.canvas}]`)) stale.remove();
  populate(state).catch((error) => {
    if (state.disposed) return;
    dispose(state);
    fail(el, error, null);
  });
}

/** Makes the application, the objects, and the observers of a stage. */
async function populate(state) {
  const { el, config } = state;
  const PIXI = globalThis.PIXI;
  if (!PIXI?.Application) throw new Error("PixiJS is not loaded. Put pixi_script() in the page <head>.");
  configure(PIXI);
  const loads = Promise.allSettled(config.objects.map((o) => load(PIXI, o)));
  const app = new PIXI.Application();
  state.app = app;
  await app.init({
    preference: PREFERENCE[config.renderer],
    background: config.background ?? 0x000000,
    backgroundAlpha: config.background === null ? 0 : 1,
    antialias: true,
    resolution: Math.min(window.devicePixelRatio || 1, MAX_RESOLUTION),
    autoDensity: false,
    autoStart: false,
    sharedTicker: false,
    width: Math.max(1, el.clientWidth),
    height: Math.max(1, el.clientHeight),
  });
  if (state.disposed) {
    app.destroy({ removeView: true }, { children: true });
    return;
  }
  state.started = true;
  const canvas = app.canvas;
  state.canvas = canvas;
  canvas.setAttribute(ATTR.canvas, "");
  canvas.setAttribute("aria-hidden", "true");
  // The browser can drop a context (for example, too many contexts).
  canvas.addEventListener("webglcontextlost", () => {
    if (state.disposed) return;
    dispose(state);
    fail(el, new Error("WebGL context lost"), null);
  });
  el.append(canvas);
  state.root = app.stage.addChild(new PIXI.Container());
  app.ticker.add((ticker) => {
    const dt = Math.min(Math.max(ticker.deltaMS / 1000, 0), MAX_DELTA_SECONDS);
    for (const step of state.motion) step(dt);
  });

  const results = await loads;
  if (state.disposed) return;
  const index = results.findIndex((r) => r.status === "rejected");
  const failure = index < 0 ? null : { error: results[index].reason, src: config.objects[index].src };
  if (failure && el.querySelector(`:scope > [${ATTR.fallback}]`)) {
    // A fallback exists: show it and free the renderer.
    dispose(state);
    fail(el, failure.error, failure.src);
    return;
  }
  config.objects.forEach((o, i) => {
    if (results[i].status === "fulfilled") place(state, make(PIXI, state, o, results[i].value), o);
  });

  const resizeObserver = new ResizeObserver(() => resize(state));
  const viewObserver = new IntersectionObserver((entries) => {
    state.visible = entries[entries.length - 1].isIntersecting;
    updateLoop(state);
  });
  resizeObserver.observe(el);
  viewObserver.observe(el);
  state.observers.push(resizeObserver, viewObserver);
  resize(state);

  el.autumnPixi = {
    PIXI,
    app,
    root: state.root,
    objects: state.objects,
    motion: state.motion,
    size: [...config.size],
    render: () => renderNow(state),
    requestRender: () => requestRender(state),
    update: () => updateLoop(state),
    get looping() {
      return state.looping;
    },
  };
  state.ready = true;
  updateLoop(state);
  if (failure) {
    // No fallback: keep the other objects on screen.
    fail(el, failure.error, failure.src);
  } else {
    el.setAttribute(ATTR.state, "ready");
    emit(el, "pixi:ready", el.autumnPixi);
  }
}

/** Stage elements in `node`, including `node` itself. */
function stagesIn(node) {
  if (node.nodeType !== Node.ELEMENT_NODE && node.nodeType !== Node.DOCUMENT_NODE) return [];
  const found = [...node.querySelectorAll(SELECTOR)];
  if (node.matches?.(SELECTOR)) found.unshift(node);
  return found;
}

/** Builds every new connected stage in `node`. */
function scan(node) {
  for (const el of stagesIn(node)) {
    if (live.has(el) || parked.has(el) || !el.isConnected) continue;
    build(el);
  }
}

/** Frees every stage in `node` that is no longer in the document. */
function sweep(node) {
  for (const el of stagesIn(node)) {
    if (el.isConnected) continue;
    parked.delete(el);
    const state = live.get(el);
    if (state) dispose(state);
  }
}

/** True when `node` is a declaration element. */
function isDeclaration(node) {
  return node.nodeType === Node.ELEMENT_NODE && KINDS.some((attr) => node.hasAttribute(attr));
}

/** The stage that `record` changes, or `null`. */
function changedStage(record) {
  const { target } = record;
  const node = target.nodeType === Node.ELEMENT_NODE ? target : target.parentElement;
  if (!node) return null;
  if (node.matches(SELECTOR)) {
    if (record.type === "attributes") return STAGE_ATTRS.has(record.attributeName) ? node : null;
    const canvas = live.get(node)?.canvas;
    const added = [...record.addedNodes].some(isDeclaration);
    const removed = [...record.removedNodes].some((n) => isDeclaration(n) || n === canvas);
    return added || removed ? node : null;
  }
  const declaration = node.closest(DECLARATION);
  const stage = declaration?.parentElement;
  if (!stage?.matches(SELECTOR)) return null;
  // Text inside a declaration counts. Attributes count only on the declaration itself.
  return record.type !== "attributes" || target === declaration ? stage : null;
}

/** Builds `el` again from its current markup. */
function rebuild(el) {
  const state = live.get(el);
  if (state) dispose(state);
  parked.delete(el);
  scan(el);
}

new MutationObserver((records) => {
  const changed = new Set();
  for (const record of records) {
    for (const node of record.removedNodes) sweep(node);
    for (const node of record.addedNodes) scan(node);
    const stage = changedStage(record);
    if (stage) changed.add(stage);
  }
  for (const el of changed) if (el.isConnected) rebuild(el);
}).observe(document.documentElement, {
  childList: true,
  subtree: true,
  characterData: true,
  attributes: true,
  attributeFilter: WATCHED,
});

document.addEventListener("htmx:afterSwap", (event) => {
  const target = event.detail?.target;
  scan(target?.isConnected ? target : document);
});

document.addEventListener("htmx:beforeCleanupElement", (event) => {
  const state = live.get(event.target);
  if (!state) return;
  dispose(state);
  parked.add(event.target);
});

reducedMotion.addEventListener("change", () => {
  for (const state of live.values()) if (state.ready) updateLoop(state);
});

// Tab turns on the PixiJS accessibility layer. It adds its buttons at the
// next render, so render the stages that have tappable objects.
document.addEventListener("keydown", (event) => {
  if (event.key !== "Tab") return;
  for (const state of live.values()) if (state.tappable) requestRender(state);
});

// First scan after DOMContentLoaded: then every deferred and module script
// on the page has run, and their `pixi:ready` listeners exist.
const navigation = performance.getEntriesByType?.("navigation")?.[0];
if (document.readyState === "loading" || navigation?.domContentLoadedEventStart === 0) {
  document.addEventListener("DOMContentLoaded", () => scan(document), { once: true });
} else {
  scan(document);
}
