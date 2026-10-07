// Unit tests for assets/parse.js. Run: npm run test:unit
import { test } from "node:test";
import assert from "node:assert/strict";

import {
  ALIGNS,
  ATTR,
  LIMITS,
  RENDERERS,
  SHAPES,
  SHAPE_DEFAULTS,
  STAGE_DEFAULTS,
  TEXT_DEFAULTS,
  SHEET_DEFAULTS,
  parseColor,
  parseKeyword,
  parseNumber,
  parseShapeArgs,
  parseSize,
  parseUrl,
  parseVec2,
  readStage,
} from "../../assets/parse.js";

/** A fake element: attributes, text, and children. */
function el(attrs = {}, children = [], textContent = "") {
  return {
    getAttribute: (name) => (name in attrs ? attrs[name] : null),
    hasAttribute: (name) => name in attrs,
    children,
    textContent,
  };
}

const BASE = "https://example.test/page";

test("parseNumber accepts finite numbers and clamps", () => {
  assert.equal(parseNumber("1.5", 0), 1.5);
  assert.equal(parseNumber(" -2 ", 0), -2);
  assert.equal(parseNumber("5", 0, 0, 1), 1);
  assert.equal(parseNumber("-5", 0, 0, 1), 0);
  assert.equal(parseNumber("1e3", 0), 1000);
});

test("parseNumber rejects bad input", () => {
  for (const bad of [null, undefined, 7, "", "abc", "NaN", "Infinity", "-Infinity", "1e999", "1px", "0x10"]) {
    assert.equal(parseNumber(bad, 7), 7, String(bad));
  }
});

test("parseVec2 reads two numbers or one number", () => {
  assert.deepEqual(parseVec2("1,2", [0, 0]), [1, 2]);
  assert.deepEqual(parseVec2(" 1 , -2.5 ", [0, 0]), [1, -2.5]);
  assert.deepEqual(parseVec2("2", [0, 0]), [2, 2]);
});

test("parseVec2 rejects bad input as a whole and copies the fallback", () => {
  for (const bad of [null, "", "1,2,3", "1,x", "NaN,0", "1,"]) {
    assert.deepEqual(parseVec2(bad, [9, 9]), [9, 9], String(bad));
  }
  const fallback = [1, 2];
  const out = parseVec2(null, fallback);
  out[0] = 5;
  assert.deepEqual(fallback, [1, 2]);
  assert.equal(parseVec2(null, null), null);
});

test("parseSize needs two sides above zero and clamps them", () => {
  assert.deepEqual(parseSize("640,360", null), [640, 360]);
  assert.deepEqual(parseSize("50", null), [50, 50]);
  assert.deepEqual(parseSize("1e9,0.5", null, 1), [LIMITS.maxSide, 1]);
  for (const bad of [null, "0,10", "-1,10", "x", "10,NaN"]) {
    assert.equal(parseSize(bad, null), null, String(bad));
  }
});

test("parseColor reads #rrggbb and #rgb", () => {
  assert.equal(parseColor("#ff8800", 0), 0xff8800);
  assert.equal(parseColor(" #F80 ", 0), 0xff8800);
  for (const bad of [null, "", "ff8800", "#ff88", "#ggg", "red", "#ff880000"]) {
    assert.equal(parseColor(bad, 7), 7, String(bad));
  }
});

test("parseKeyword is case-insensitive and falls back", () => {
  assert.equal(parseKeyword(" Canvas ", RENDERERS, "auto"), "canvas");
  assert.equal(parseKeyword("metal", RENDERERS, "auto"), "auto");
  assert.equal(parseKeyword(null, ALIGNS, "left"), "left");
});

test("parseUrl allows http(s) and relative URLs only", () => {
  assert.equal(parseUrl("/a.png", BASE), "https://example.test/a.png");
  assert.equal(parseUrl("b.png", BASE), "https://example.test/b.png");
  assert.equal(parseUrl("http://cdn.test/c.png", BASE), "http://cdn.test/c.png");
  for (const bad of [null, "", "  ", "javascript:alert(1)", "data:image/png;base64,AA", "blob:x", "http://[bad"]) {
    assert.equal(parseUrl(bad, BASE), null, String(bad));
  }
});

test("parseShapeArgs uses per-kind defaults for missing or bad sizes", () => {
  assert.deepEqual(parseShapeArgs("rect", "10,20"), [10, 20]);
  assert.deepEqual(parseShapeArgs("rect", null), SHAPES.rect);
  assert.deepEqual(parseShapeArgs("rounded-rect", "10,x,-1"), [10, 100, 12]);
  assert.deepEqual(parseShapeArgs("circle", "5"), [5]);
  assert.deepEqual(parseShapeArgs("ellipse", "6"), [6, 40]);
  assert.equal(parseShapeArgs("hexagon", "1"), null);
  assert.equal(parseShapeArgs("__proto__", "1"), null);
});

test("parseShapeArgs rounds and clamps star points", () => {
  assert.deepEqual(parseShapeArgs("star", "6.6,50,20"), [7, 50, 20]);
  assert.deepEqual(parseShapeArgs("star", "1,50,20"), [3, 50, 20]);
  assert.deepEqual(parseShapeArgs("star", "100000,50,20"), [100, 50, 20]);
});

test("parseShapeArgs reads polygon corners and caps them", () => {
  assert.deepEqual(parseShapeArgs("polygon", "0,0,10,0,5,8"), [0, 0, 10, 0, 5, 8]);
  assert.deepEqual(parseShapeArgs("polygon", "0,0,10,0"), SHAPES.polygon, "too few corners");
  assert.deepEqual(parseShapeArgs("polygon", "0,0,10,0,5"), SHAPES.polygon, "odd count");
  assert.deepEqual(parseShapeArgs("polygon", "0,0,x,0,5,5"), SHAPES.polygon, "bad number");
  assert.deepEqual(parseShapeArgs("polygon", "-5,-5,5,-5,0,5"), [-5, -5, 5, -5, 0, 5], "negative corners");
  const many = Array.from({ length: 2000 }, (_, i) => i).join(",");
  assert.equal(parseShapeArgs("polygon", many).length, LIMITS.maxPolygonPoints * 2);
});

test("readStage reads defaults from a bare element", () => {
  const config = readStage(el({ [ATTR.stage]: "stage" }), BASE);
  assert.deepEqual(config, {
    size: [...STAGE_DEFAULTS.size],
    background: null,
    renderer: "auto",
    reducedAnimate: false,
    objects: [],
    warnings: [],
  });
});

test("readStage reads every stage attribute", () => {
  const config = readStage(
    el({
      [ATTR.size]: "640,360",
      [ATTR.background]: "#112233",
      [ATTR.renderer]: "canvas",
      [ATTR.reduced]: "animate",
    }),
    BASE,
  );
  assert.deepEqual(config.size, [640, 360]);
  assert.equal(config.background, 0x112233);
  assert.equal(config.renderer, "canvas");
  assert.equal(config.reducedAnimate, true);
});

test("readStage falls back for bad stage attributes", () => {
  const config = readStage(
    el({ [ATTR.size]: "0,10", [ATTR.background]: "blue", [ATTR.renderer]: "gpu", [ATTR.reduced]: "yes" }),
    BASE,
  );
  assert.deepEqual(config.size, [...STAGE_DEFAULTS.size]);
  assert.equal(config.background, null);
  assert.equal(config.renderer, "auto");
  assert.equal(config.reducedAnimate, false);
});

test("a sprite reads every common attribute", () => {
  const child = el({
    id: "coin",
    [ATTR.sprite]: "/coin.png",
    [ATTR.size]: "32,16",
    [ATTR.label]: "Gold coin",
    [ATTR.position]: "10,20",
    [ATTR.rotation]: "45",
    [ATTR.scale]: "2",
    [ATTR.anchor]: "0,1",
    [ATTR.alpha]: "0.5",
    [ATTR.tint]: "#ff0000",
    [ATTR.spin]: "-90",
    [ATTR.tap]: "true",
  });
  const [sprite] = readStage(el({}, [child]), BASE).objects;
  assert.deepEqual(sprite, {
    type: "sprite",
    source: child,
    id: "coin",
    label: "Gold coin",
    src: "https://example.test/coin.png",
    size: [32, 16],
    position: [10, 20],
    rotation: 45,
    scale: [2, 2],
    anchor: [0, 1],
    alpha: 0.5,
    tint: 0xff0000,
    spin: -90,
    tap: true,
  });
});

test("common attributes have defaults", () => {
  const [shape] = readStage(el({}, [el({ [ATTR.shape]: "circle" })]), BASE).objects;
  assert.equal(shape.id, null);
  assert.equal(shape.label, null);
  assert.equal(shape.position, null, "null means the stage center");
  assert.equal(shape.rotation, 0);
  assert.deepEqual(shape.scale, [1, 1]);
  assert.deepEqual(shape.anchor, [0.5, 0.5]);
  assert.equal(shape.alpha, 1);
  assert.equal(shape.tint, null);
  assert.equal(shape.spin, 0);
  assert.equal(shape.tap, false);
});

test("alpha clamps and tap needs the exact value true", () => {
  const read = (attrs) => readStage(el({}, [el({ [ATTR.shape]: "rect", ...attrs })]), BASE).objects[0];
  assert.equal(read({ [ATTR.alpha]: "7" }).alpha, 1);
  assert.equal(read({ [ATTR.alpha]: "-1" }).alpha, 0);
  assert.equal(read({ [ATTR.tap]: "yes" }).tap, false);
  assert.equal(read({ [ATTR.label]: "  " }).label, null);
  assert.equal(read({ id: "" }).id, null);
});

test("a tiling sprite reads its size and scroll", () => {
  const [tiling] = readStage(
    el({}, [el({ [ATTR.tiling]: "/sky.png", [ATTR.size]: "640,360", [ATTR.scroll]: "-30,5" })]),
    BASE,
  ).objects;
  assert.equal(tiling.type, "tiling");
  assert.equal(tiling.src, "https://example.test/sky.png");
  assert.deepEqual(tiling.size, [640, 360]);
  assert.deepEqual(tiling.scroll, [-30, 5]);
  const [plain] = readStage(el({}, [el({ [ATTR.tiling]: "/sky.png" })]), BASE).objects;
  assert.equal(plain.size, null, "null means the stage size");
  assert.deepEqual(plain.scroll, [0, 0]);
});

test("an animated sprite reads its animation and speed", () => {
  const [sheet] = readStage(
    el({}, [el({ [ATTR.sheet]: "/bird.json", [ATTR.animation]: " flap ", [ATTR.fps]: "500" })]),
    BASE,
  ).objects;
  assert.equal(sheet.type, "sheet");
  assert.equal(sheet.src, "https://example.test/bird.json");
  assert.equal(sheet.animation, "flap");
  assert.equal(sheet.fps, LIMITS.fps[1]);
  const [plain] = readStage(el({}, [el({ [ATTR.sheet]: "/bird.json", [ATTR.animation]: "" })]), BASE).objects;
  assert.equal(plain.animation, null, "null means the first animation");
  assert.equal(plain.fps, SHEET_DEFAULTS.fps);
});

test("a text object reads its content and style", () => {
  const [text] = readStage(
    el({}, [
      el(
        {
          [ATTR.text]: "",
          [ATTR.fontSize]: "32",
          [ATTR.fontFamily]: " Georgia, serif ",
          [ATTR.fill]: "#ffffff",
          [ATTR.weight]: "BOLD",
          [ATTR.align]: "center",
          [ATTR.wrap]: "300",
        },
        [],
        "\n  Score: 10\n",
      ),
    ]),
    BASE,
  ).objects;
  assert.equal(text.type, "text");
  assert.equal(text.text, "Score: 10");
  assert.equal(text.fontSize, 32);
  assert.equal(text.fontFamily, "Georgia, serif");
  assert.equal(text.fill, 0xffffff);
  assert.equal(text.weight, "bold");
  assert.equal(text.align, "center");
  assert.equal(text.wrap, 300);
});

test("text style has defaults and limits", () => {
  const long = "x".repeat(LIMITS.maxText + 50);
  const [text] = readStage(
    el({}, [el({ [ATTR.text]: "", [ATTR.fontSize]: "0", [ATTR.wrap]: "-5", [ATTR.fontFamily]: "" }, [], long)]),
    BASE,
  ).objects;
  assert.equal(text.text.length, LIMITS.maxText);
  assert.equal(text.fontSize, LIMITS.fontSize[0]);
  assert.equal(text.fontFamily, TEXT_DEFAULTS.fontFamily);
  assert.equal(text.fill, TEXT_DEFAULTS.fill);
  assert.equal(text.weight, "normal");
  assert.equal(text.align, "left");
  assert.equal(text.wrap, null);
});

test("a shape reads its geometry, fill, and stroke", () => {
  const [shape] = readStage(
    el({}, [
      el({
        [ATTR.shape]: "Rounded-Rect",
        [ATTR.args]: "100,50,8",
        [ATTR.fill]: "#123456",
        [ATTR.stroke]: "#000000",
        [ATTR.strokeWidth]: "3",
      }),
    ]),
    BASE,
  ).objects;
  assert.equal(shape.type, "shape");
  assert.equal(shape.kind, "rounded-rect");
  assert.deepEqual(shape.args, [100, 50, 8]);
  assert.equal(shape.fill, 0x123456);
  assert.equal(shape.stroke, 0x000000);
  assert.equal(shape.strokeWidth, 3);
});

test("shape fill and stroke have defaults", () => {
  const read = (attrs) => readStage(el({}, [el({ [ATTR.shape]: "rect", ...attrs })]), BASE).objects[0];
  assert.equal(read({}).fill, SHAPE_DEFAULTS.fill);
  assert.equal(read({ [ATTR.fill]: "none" }).fill, null, "none means no fill");
  assert.equal(read({}).stroke, null);
  assert.equal(read({ [ATTR.stroke]: "#fff" }).strokeWidth, SHAPE_DEFAULTS.strokeWidth);
  assert.equal(read({ [ATTR.stroke]: "#fff", [ATTR.strokeWidth]: "-1" }).strokeWidth, SHAPE_DEFAULTS.strokeWidth);
});

test("bad declarations are skipped with a warning; other children are ignored", () => {
  const config = readStage(
    el({}, [
      el({ [ATTR.shape]: "teapot" }),
      el({ [ATTR.sprite]: "javascript:alert(1)" }),
      el({ [ATTR.tiling]: "" }),
      el({ [ATTR.sheet]: "data:x" }),
      el({ [ATTR.fallback]: "" }),
      el({ class: "unrelated" }),
      el({ [ATTR.shape]: "circle" }),
    ]),
    BASE,
  );
  assert.equal(config.objects.length, 1);
  assert.equal(config.warnings.length, 4);
  assert.match(config.warnings[0], /teapot/);
  assert.match(config.warnings[1], /javascript/);
});

test("objects keep declaration order", () => {
  const config = readStage(
    el({}, [
      el({ [ATTR.text]: "" }, [], "a"),
      el({ [ATTR.shape]: "rect" }),
      el({ [ATTR.sprite]: "/s.png" }),
    ]),
    BASE,
  );
  assert.deepEqual(config.objects.map((o) => o.type), ["text", "shape", "sprite"]);
});

test("readStage tolerates a missing children list", () => {
  const config = readStage({ getAttribute: () => null }, BASE);
  assert.deepEqual(config.objects, []);
});

test("every attribute name is unique and uses the plugin prefix", () => {
  const names = Object.values(ATTR);
  assert.equal(new Set(names).size, names.length);
  for (const name of names) assert.match(name, /^data-pixi(-[a-z]+)*$/);
});

test("exported tables are frozen", () => {
  for (const table of [ATTR, SHAPES, LIMITS, STAGE_DEFAULTS, TEXT_DEFAULTS, SHAPE_DEFAULTS, SHEET_DEFAULTS, RENDERERS, ALIGNS]) {
    assert.ok(Object.isFrozen(table));
  }
});
