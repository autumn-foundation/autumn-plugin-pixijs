// autumn-plugin-pixijs: attribute parsers.
//
// This file has only pure functions. It does not use PixiJS or DOM
// globals. init.js uses it. tests/js runs it in Node. Bad input never
// throws: each parser returns its fallback.

/** Every attribute the plugin reads or writes. */
export const ATTR = Object.freeze({
  stage: "data-pixi",
  size: "data-pixi-size",
  background: "data-pixi-background",
  renderer: "data-pixi-renderer",
  reduced: "data-pixi-reduced",
  fallback: "data-pixi-fallback",
  state: "data-pixi-state",
  canvas: "data-pixi-canvas",
  sprite: "data-pixi-sprite",
  tiling: "data-pixi-tiling",
  sheet: "data-pixi-sheet",
  text: "data-pixi-text",
  shape: "data-pixi-shape",
  args: "data-pixi-args",
  scroll: "data-pixi-scroll",
  animation: "data-pixi-animation",
  fps: "data-pixi-fps",
  fill: "data-pixi-fill",
  stroke: "data-pixi-stroke",
  strokeWidth: "data-pixi-stroke-width",
  fontSize: "data-pixi-font-size",
  fontFamily: "data-pixi-font-family",
  weight: "data-pixi-weight",
  align: "data-pixi-align",
  wrap: "data-pixi-wrap",
  label: "data-pixi-label",
  position: "data-pixi-position",
  rotation: "data-pixi-rotation",
  scale: "data-pixi-scale",
  anchor: "data-pixi-anchor",
  alpha: "data-pixi-alpha",
  tint: "data-pixi-tint",
  spin: "data-pixi-spin",
  tap: "data-pixi-tap",
});

/** `data-pixi` value of a stage element. */
export const STAGE = "stage";

/** Limits that keep bad markup from stopping the page. */
export const LIMITS = Object.freeze({
  maxSide: 8192,
  maxPolygonPoints: 512,
  starPoints: Object.freeze([3, 100]),
  fontSize: Object.freeze([1, 512]),
  fps: Object.freeze([1, 120]),
  maxText: 10000,
  maxFontFamily: 200,
});

/** Default sizes per shape, in `data-pixi-args` order. */
export const SHAPES = Object.freeze({
  "rect": Object.freeze([100, 100]),
  "rounded-rect": Object.freeze([100, 100, 12]),
  "circle": Object.freeze([50]),
  "ellipse": Object.freeze([60, 40]),
  "star": Object.freeze([5, 50, 25]),
  "polygon": Object.freeze([0, -50, 50, 50, -50, 50]),
});

/** Allowed `data-pixi-renderer` values. The first is the default. */
export const RENDERERS = Object.freeze(["auto", "webgl", "canvas"]);

/** Allowed `data-pixi-align` values. The first is the default. */
export const ALIGNS = Object.freeze(["left", "center", "right"]);

/** Allowed `data-pixi-weight` values. The first is the default. */
export const WEIGHTS = Object.freeze(["normal", "bold"]);

/** `data-pixi-reduced` value that keeps motion for reduced-motion users. */
export const REDUCED_ANIMATE = "animate";

/** `data-pixi-fill` value for no fill. */
export const NO_FILL = "none";

/** `data-pixi-tap` value that makes an object tappable. */
export const TAP = "true";

/** Stage defaults. */
export const STAGE_DEFAULTS = Object.freeze({ size: Object.freeze([800, 450]) });

/** Text defaults. */
export const TEXT_DEFAULTS = Object.freeze({ fontSize: 24, fontFamily: "sans-serif", fill: 0x000000 });

/** Shape defaults. */
export const SHAPE_DEFAULTS = Object.freeze({ fill: 0xffffff, strokeWidth: 2 });

/** Animated sprite defaults. */
export const SHEET_DEFAULTS = Object.freeze({ fps: 12 });

const NUMBER = /^[+-]?(\d+\.?\d*|\.\d+)(e[+-]?\d+)?$/i;
const HEX6 = /^#([0-9a-f]{6})$/i;
const HEX3 = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i;

/** Parses a finite decimal number, clamped to `[min, max]`. */
export function parseNumber(value, fallback, min = -Infinity, max = Infinity) {
  if (typeof value !== "string") return fallback;
  const text = value.trim();
  if (!NUMBER.test(text)) return fallback;
  const n = Number(text);
  if (!Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

/** Parses `x,y` or one number for both. Bad input returns a copy of `fallback`. */
export function parseVec2(value, fallback) {
  if (typeof value === "string") {
    const parts = value.split(",");
    if (parts.length === 1 || parts.length === 2) {
      const nums = parts.map((p) => parseNumber(p, NaN));
      if (nums.every(Number.isFinite)) return nums.length === 1 ? [nums[0], nums[0]] : nums;
    }
  }
  return fallback === null ? null : [...fallback];
}

/** Parses `w,h` (or one side). Each side must be above zero. Sides clamp to `[min, maxSide]`. */
export function parseSize(value, fallback, min = 0) {
  const size = parseVec2(value, null);
  if (!size || !(size[0] > 0 && size[1] > 0)) return fallback;
  return size.map((side) => Math.min(LIMITS.maxSide, Math.max(min, side)));
}

/** Parses `#rrggbb` or `#rgb` to `0xrrggbb`. */
export function parseColor(value, fallback) {
  if (typeof value !== "string") return fallback;
  const text = value.trim();
  const six = HEX6.exec(text);
  if (six) return Number.parseInt(six[1], 16);
  const three = HEX3.exec(text);
  if (three) return Number.parseInt(three[1] + three[1] + three[2] + three[2] + three[3] + three[3], 16);
  return fallback;
}

/** Returns the lower-case `value` when `allowed` has it, else `fallback`. */
export function parseKeyword(value, allowed, fallback) {
  if (typeof value !== "string") return fallback;
  const key = value.trim().toLowerCase();
  return allowed.includes(key) ? key : fallback;
}

/** Resolves a URL against `base`. Only `http:` and `https:` pass. */
export function parseUrl(value, base) {
  if (typeof value !== "string" || value.trim() === "") return null;
  try {
    const url = new URL(value.trim(), base);
    return url.protocol === "http:" || url.protocol === "https:" ? url.href : null;
  } catch {
    return null;
  }
}

/** Parses polygon corners `x1,y1,x2,y2,...`. Bad input: the default triangle. */
function parsePolygon(value) {
  const parts = typeof value === "string" ? value.split(",") : [];
  const nums = parts.slice(0, LIMITS.maxPolygonPoints * 2).map((p) => parseNumber(p, NaN));
  const valid = nums.length >= 6 && nums.length % 2 === 0 && nums.every(Number.isFinite);
  return valid ? nums : [...SHAPES.polygon];
}

/** Parses shape sizes. Missing or bad sizes get defaults. Unknown kind: `null`. */
export function parseShapeArgs(kind, value) {
  const defaults = Object.hasOwn(SHAPES, kind) ? SHAPES[kind] : null;
  if (!defaults) return null;
  if (kind === "polygon") return parsePolygon(value);
  const parts = typeof value === "string" ? value.split(",") : [];
  const args = defaults.map((d, i) => {
    const n = parseNumber(parts[i], NaN);
    return n >= 0 ? Math.min(n, LIMITS.maxSide) : d;
  });
  if (kind === "star") {
    const [min, max] = LIMITS.starPoints;
    args[0] = Math.min(max, Math.max(min, Math.round(args[0])));
  }
  return args;
}

/** Returns trimmed text, or `null` when it is empty. */
function text(value) {
  const trimmed = typeof value === "string" ? value.trim() : "";
  return trimmed === "" ? null : trimmed;
}

/** Reads the settings that all objects share. */
function readCommon(child) {
  return {
    source: child,
    id: text(child.getAttribute("id")),
    label: text(child.getAttribute(ATTR.label)),
    position: parseVec2(child.getAttribute(ATTR.position), null),
    rotation: parseNumber(child.getAttribute(ATTR.rotation), 0),
    scale: parseVec2(child.getAttribute(ATTR.scale), [1, 1]),
    anchor: parseVec2(child.getAttribute(ATTR.anchor), [0.5, 0.5]),
    alpha: parseNumber(child.getAttribute(ATTR.alpha), 1, 0, 1),
    tint: parseColor(child.getAttribute(ATTR.tint), null),
    spin: parseNumber(child.getAttribute(ATTR.spin), 0),
    tap: child.getAttribute(ATTR.tap) === TAP,
  };
}

/** Reads an image sprite, or `null` for a bad URL. */
function readSprite(child, base) {
  const src = parseUrl(child.getAttribute(ATTR.sprite), base);
  if (!src) return null;
  return { type: "sprite", ...readCommon(child), src, size: parseSize(child.getAttribute(ATTR.size), null) };
}

/** Reads a tiling sprite, or `null` for a bad URL. */
function readTiling(child, base) {
  const src = parseUrl(child.getAttribute(ATTR.tiling), base);
  if (!src) return null;
  return {
    type: "tiling",
    ...readCommon(child),
    src,
    size: parseSize(child.getAttribute(ATTR.size), null),
    scroll: parseVec2(child.getAttribute(ATTR.scroll), [0, 0]),
  };
}

/** Reads an animated sprite, or `null` for a bad URL. */
function readSheet(child, base) {
  const src = parseUrl(child.getAttribute(ATTR.sheet), base);
  if (!src) return null;
  return {
    type: "sheet",
    ...readCommon(child),
    src,
    animation: text(child.getAttribute(ATTR.animation)),
    fps: parseNumber(child.getAttribute(ATTR.fps), SHEET_DEFAULTS.fps, ...LIMITS.fps),
  };
}

/** Reads a text object. The text is the element content. */
function readText(child) {
  const family = text(child.getAttribute(ATTR.fontFamily));
  const wrap = parseNumber(child.getAttribute(ATTR.wrap), NaN, 0, LIMITS.maxSide);
  return {
    type: "text",
    ...readCommon(child),
    text: String(child.textContent ?? "").trim().slice(0, LIMITS.maxText),
    fontSize: parseNumber(child.getAttribute(ATTR.fontSize), TEXT_DEFAULTS.fontSize, ...LIMITS.fontSize),
    fontFamily: family ? family.slice(0, LIMITS.maxFontFamily) : TEXT_DEFAULTS.fontFamily,
    fill: parseColor(child.getAttribute(ATTR.fill), TEXT_DEFAULTS.fill),
    weight: parseKeyword(child.getAttribute(ATTR.weight), WEIGHTS, WEIGHTS[0]),
    align: parseKeyword(child.getAttribute(ATTR.align), ALIGNS, ALIGNS[0]),
    wrap: wrap > 0 ? wrap : null,
  };
}

/** Reads a shape, or `null` for an unknown kind. */
function readShape(child) {
  const kind = child.getAttribute(ATTR.shape).trim().toLowerCase();
  const args = parseShapeArgs(kind, child.getAttribute(ATTR.args));
  if (!args) return null;
  const fill = child.getAttribute(ATTR.fill);
  const stroke = parseColor(child.getAttribute(ATTR.stroke), null);
  const width = parseNumber(child.getAttribute(ATTR.strokeWidth), NaN, 0, LIMITS.maxSide);
  return {
    type: "shape",
    ...readCommon(child),
    kind,
    args,
    fill: fill?.trim().toLowerCase() === NO_FILL ? null : parseColor(fill, SHAPE_DEFAULTS.fill),
    stroke,
    strokeWidth: width > 0 ? width : SHAPE_DEFAULTS.strokeWidth,
  };
}

/** Declaration kinds: marker attribute and reader, in precedence order. */
const KINDS = [
  [ATTR.sprite, readSprite],
  [ATTR.tiling, readTiling],
  [ATTR.sheet, readSheet],
  [ATTR.text, readText],
  [ATTR.shape, readShape],
];

/**
 * Reads a stage element into a plain config object.
 *
 * `el` needs `getAttribute` and `children`. Each child needs
 * `getAttribute`, `hasAttribute`, and `textContent`. Bad declarations are
 * skipped and named in `warnings`.
 */
export function readStage(el, base) {
  const warnings = [];
  const objects = [];
  for (const child of el.children ?? []) {
    const kind = KINDS.find(([attr]) => child.hasAttribute(attr));
    if (!kind) continue;
    const [attr, read] = kind;
    const object = read(child, base);
    if (object) {
      objects.push(object);
    } else {
      warnings.push(`ignored ${attr}="${child.getAttribute(attr)}"`);
    }
  }
  return {
    size: parseSize(el.getAttribute(ATTR.size), [...STAGE_DEFAULTS.size], 1),
    background: parseColor(el.getAttribute(ATTR.background), null),
    renderer: parseKeyword(el.getAttribute(ATTR.renderer), RENDERERS, RENDERERS[0]),
    reducedAnimate: el.getAttribute(ATTR.reduced) === REDUCED_ANIMATE,
    objects,
    warnings,
  };
}
