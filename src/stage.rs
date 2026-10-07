//! The typed stage builder: [`Stage`] and its display objects.
//!
//! A [`Stage`] renders one `<div data-pixi="stage">` element. Each object
//! renders one hidden child declaration (`data-pixi-sprite`,
//! `data-pixi-tiling`, `data-pixi-sheet`, `data-pixi-text`,
//! `data-pixi-shape`). `init.js` reads the markup and builds the PixiJS
//! scene. You can also write the markup by hand. The README gives the
//! attribute reference.
//!
//! Lengths use logical stage pixels. The runtime scales the stage to the
//! element. Angles use degrees. Spin uses degrees per second. Scroll uses
//! logical pixels per second.
//!
//! The builder never writes a non-finite number. A setter ignores `NaN` and
//! infinite input. Setters clamp sizes to [`MAX_STAGE_SIDE`] and other
//! values to the ranges in their docs. The runtime applies the same limits.

use autumn_web::{Markup, html};
use maud::Render;

/// Default logical stage size: `800 × 450`.
pub const DEFAULT_STAGE_SIZE: Vec2 = Vec2::new(800.0, 450.0);
/// Largest stage side, in logical pixels.
pub const MAX_STAGE_SIDE: f32 = 8192.0;
/// Maximum number of polygon corners. The builder drops the extra corners.
pub const MAX_POLYGON_POINTS: usize = 512;
/// Minimum and maximum number of star points.
const STAR_POINTS: (u32, u32) = (3, 100);
/// Smallest and largest font size, in logical pixels.
const FONT_SIZE: (f32, f32) = (1.0, 512.0);
/// Smallest and largest animation speed, in frames per second.
const FPS: (f32, f32) = (1.0, 120.0);
/// Default sizes per shape kind, in `data-pixi-args` order. `parse.js`
/// has the same table (a test checks it).
const SHAPE_DEFAULTS: [(&str, &[f32]); 6] = [
    ("rect", &[100.0, 100.0]),
    ("rounded-rect", &[100.0, 100.0, 12.0]),
    ("circle", &[50.0]),
    ("ellipse", &[60.0, 40.0]),
    ("star", &[5.0, 50.0, 25.0]),
    ("polygon", &[0.0, -50.0, 50.0, 50.0, -50.0, 50.0]),
];

/// Returns `Some(value)` when it is finite, else `None`.
const fn finite(value: f32) -> Option<f32> {
    if value.is_finite() { Some(value) } else { None }
}

/// Returns `Some(value)` when it is finite and above zero, else `None`.
/// The value clamps to [`MAX_STAGE_SIDE`].
fn positive(value: f32) -> Option<f32> {
    finite(value)
        .filter(|v| *v > 0.0)
        .map(|v| v.min(MAX_STAGE_SIDE))
}

/// Formats a finite number for an attribute. `-0` becomes `0`.
fn num(value: f32) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        format!("{value}")
    }
}

/// A 2D vector: position, scale, anchor, size, or scroll speed.
///
/// ```rust
/// use autumn_plugin_pixijs::Vec2;
///
/// assert_eq!(Vec2::from([1.0, 2.0]), Vec2::new(1.0, 2.0));
/// assert_eq!(Vec2::splat(2.0), Vec2::new(2.0, 2.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    /// X component (right is positive).
    pub x: f32,
    /// Y component (down is positive).
    pub y: f32,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Makes a vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Makes a vector with both components set to `value`.
    #[must_use]
    pub const fn splat(value: f32) -> Self {
        Self::new(value, value)
    }

    /// Returns `Some(self)` when both components are finite.
    const fn finite(self) -> Option<Self> {
        if self.x.is_finite() && self.y.is_finite() {
            Some(self)
        } else {
            None
        }
    }

    /// The `x,y` attribute value.
    fn attr(self) -> String {
        format!("{},{}", num(self.x), num(self.y))
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from([x, y]: [f32; 2]) -> Self {
        Self::new(x, y)
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl From<f32> for Vec2 {
    /// One value for both components, for example a uniform scale.
    fn from(value: f32) -> Self {
        Self::splat(value)
    }
}

/// An sRGB color.
///
/// ```rust
/// use autumn_plugin_pixijs::Color;
///
/// assert_eq!(Color::hex(0xff8800).to_string(), "#ff8800");
/// assert_eq!(Color::rgb(0, 128, 255).to_string(), "#0080ff");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color(u32);

impl Color {
    /// White (`#ffffff`).
    pub const WHITE: Self = Self(0x00ff_ffff);
    /// Black (`#000000`).
    pub const BLACK: Self = Self(0);

    /// Makes a color from `0xRRGGBB`. The method ignores the bits above the
    /// low 24 bits.
    #[must_use]
    pub const fn hex(value: u32) -> Self {
        Self(value & 0x00ff_ffff)
    }

    /// Makes a color from red, green, and blue channels.
    #[must_use]
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(((red as u32) << 16) | ((green as u32) << 8) | blue as u32)
    }

    /// The color as `0xRRGGBB`.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:06x}", self.0)
    }
}

/// The renderer of a [`Stage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Renderer {
    /// WebGL. When WebGL is not available, Canvas 2D (`auto`).
    #[default]
    Auto,
    /// WebGL only. Without WebGL, the stage shows its fallback (`webgl`).
    WebGl,
    /// Canvas 2D only (`canvas`). It uses no WebGL context.
    Canvas,
}

impl Renderer {
    /// The `data-pixi-renderer` value.
    const fn attr(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::WebGl => "webgl",
            Self::Canvas => "canvas",
        }
    }
}

/// Text alignment of a multi-line [`Text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Align {
    /// Left (`left`). The default.
    #[default]
    Left,
    /// Center (`center`).
    Center,
    /// Right (`right`).
    Right,
}

impl Align {
    /// The `data-pixi-align` value.
    const fn attr(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }
}

/// The htmx verb of a tap request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Get,
    Post,
}

/// An htmx request that a tap sends. Without a verb, the options do nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct TapRequest {
    verb: Option<(Verb, String)>,
    target: Option<String>,
    swap: Option<String>,
    vals: Option<String>,
}

/// Settings that all display objects share.
#[derive(Debug, Clone, PartialEq, Default)]
struct Common {
    id: Option<String>,
    label: Option<String>,
    position: Option<Vec2>,
    rotation: Option<f32>,
    scale: Option<Vec2>,
    anchor: Option<Vec2>,
    alpha: Option<f32>,
    tint: Option<Color>,
    spin: Option<f32>,
    tap: bool,
    request: Option<TapRequest>,
}

impl Common {
    /// The htmx request, made on first use.
    fn request(&mut self) -> &mut TapRequest {
        self.request.get_or_insert_with(TapRequest::default)
    }

    /// The request when it has a verb.
    fn sent_request(&self) -> Option<&TapRequest> {
        self.request.as_ref().filter(|r| r.verb.is_some())
    }

    /// The URL of the request when its verb is `verb`.
    fn url(&self, verb: Verb) -> Option<&str> {
        self.sent_request()
            .and_then(|r| r.verb.as_ref())
            .filter(|(v, _)| *v == verb)
            .map(|(_, url)| url.as_str())
    }
}

/// Adds the shared setters to a display object builder.
macro_rules! common_setters {
    () => {
        /// Sets the element `id` of the declaration. The runtime also sets
        /// it as the PixiJS `label`, so `root.getChildByLabel(id)` finds
        /// the object.
        pub fn id(mut self, id: impl Into<String>) -> Self {
            self.common.id = Some(id.into());
            self
        }

        /// Sets the accessible name of a tappable object: the label of its
        /// focusable button. Default: the `id`. Other objects ignore it.
        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.common.label = Some(label.into());
            self
        }

        /// Sets the position of the anchor point, in logical stage pixels.
        /// Default: the stage center. The method ignores a vector with a
        /// non-finite component.
        pub fn position(mut self, position: impl Into<Vec2>) -> Self {
            if let Some(v) = position.into().finite() {
                self.common.position = Some(v);
            }
            self
        }

        /// Sets the rotation in degrees (clockwise).
        pub const fn rotation(mut self, degrees: f32) -> Self {
            if let Some(v) = finite(degrees) {
                self.common.rotation = Some(v);
            }
            self
        }

        /// Sets the scale. Use [`Vec2::splat`] for a uniform scale.
        pub fn scale(mut self, scale: impl Into<Vec2>) -> Self {
            if let Some(v) = scale.into().finite() {
                self.common.scale = Some(v);
            }
            self
        }

        /// Sets the anchor: `0,0` is the top-left corner and `1,1` is the
        /// bottom-right corner. Default: `0.5,0.5` (the center).
        pub fn anchor(mut self, anchor: impl Into<Vec2>) -> Self {
            if let Some(v) = anchor.into().finite() {
                self.common.anchor = Some(v);
            }
            self
        }

        /// Sets the opacity, `0` to `1`.
        pub const fn alpha(mut self, alpha: f32) -> Self {
            if let Some(v) = finite(alpha) {
                self.common.alpha = Some(v.clamp(0.0, 1.0));
            }
            self
        }

        /// Multiplies the object colors by `color`.
        pub const fn tint(mut self, color: Color) -> Self {
            self.common.tint = Some(color);
            self
        }

        /// Turns the object around its anchor, in degrees per second.
        ///
        /// Spin stops when the user prefers reduced motion, unless the stage
        /// calls [`Stage::animate_reduced_motion`].
        pub const fn spin(mut self, degrees_per_second: f32) -> Self {
            if let Some(v) = finite(degrees_per_second) {
                self.common.spin = Some(v);
            }
            self
        }

        /// Makes the object tappable. A tap (click, touch, or Enter on the
        /// focused accessible button) sends a bubbling `pixi:tap` event from
        /// the declaration element.
        pub const fn tappable(mut self) -> Self {
            self.common.tap = true;
            self
        }

        /// Sends an htmx `GET` to `url` on each tap. Needs htmx on the page.
        /// The object becomes tappable. It replaces an earlier
        /// [`tap_post`](Self::tap_post).
        pub fn tap_get(mut self, url: impl Into<String>) -> Self {
            self.common.tap = true;
            self.common.request().verb = Some((Verb::Get, url.into()));
            self
        }

        /// Sends an htmx `POST` to `url` on each tap. Needs htmx on the
        /// page. The object becomes tappable. It replaces an earlier
        /// [`tap_get`](Self::tap_get). With CSRF on, the page needs the
        /// Autumn htmx CSRF helper (see the README).
        pub fn tap_post(mut self, url: impl Into<String>) -> Self {
            self.common.tap = true;
            self.common.request().verb = Some((Verb::Post, url.into()));
            self
        }

        /// Sets the htmx target (`hx-target`) of the tap request. It needs
        /// [`tap_get`](Self::tap_get) or [`tap_post`](Self::tap_post).
        pub fn tap_target(mut self, selector: impl Into<String>) -> Self {
            self.common.request().target = Some(selector.into());
            self
        }

        /// Sets the htmx swap strategy (`hx-swap`) of the tap request. It
        /// needs [`tap_get`](Self::tap_get) or [`tap_post`](Self::tap_post).
        pub fn tap_swap(mut self, swap: impl Into<String>) -> Self {
            self.common.request().swap = Some(swap.into());
            self
        }

        /// Sets the htmx values (`hx-vals`, a JSON object) of the tap
        /// request. It needs [`tap_get`](Self::tap_get) or
        /// [`tap_post`](Self::tap_post). Do not use the `js:` prefix: the
        /// default CSP blocks it.
        pub fn tap_vals(mut self, json: impl Into<String>) -> Self {
            self.common.request().vals = Some(json.into());
            self
        }
    };
}

/// The kind-specific attributes of one declaration.
#[derive(Debug, Default)]
struct Decl<'a> {
    sprite: Option<&'a str>,
    tiling: Option<&'a str>,
    sheet: Option<&'a str>,
    text: Option<&'a str>,
    shape: Option<&'static str>,
    args: Option<String>,
    size: Option<Vec2>,
    scroll: Option<Vec2>,
    animation: Option<&'a str>,
    fps: Option<f32>,
    fill: Option<String>,
    stroke: Option<Color>,
    stroke_width: Option<f32>,
    font_size: Option<f32>,
    font_family: Option<&'a str>,
    weight: Option<&'static str>,
    align: Option<Align>,
    wrap: Option<f32>,
}

/// Renders one hidden declaration element.
fn declaration(d: &Decl<'_>, c: &Common) -> Markup {
    let r = c.sent_request();
    html! {
        div hidden id=[c.id.as_deref()]
            data-pixi-sprite=[d.sprite]
            data-pixi-tiling=[d.tiling]
            data-pixi-sheet=[d.sheet]
            data-pixi-text=[d.text.map(|_| "")]
            data-pixi-shape=[d.shape]
            data-pixi-args=[d.args.as_deref()]
            data-pixi-size=[d.size.map(Vec2::attr)]
            data-pixi-scroll=[d.scroll.map(Vec2::attr)]
            data-pixi-animation=[d.animation]
            data-pixi-fps=[d.fps.map(num)]
            data-pixi-fill=[d.fill.as_deref()]
            data-pixi-stroke=[d.stroke]
            data-pixi-stroke-width=[d.stroke_width.map(num)]
            data-pixi-font-size=[d.font_size.map(num)]
            data-pixi-font-family=[d.font_family]
            data-pixi-weight=[d.weight]
            data-pixi-align=[d.align.map(Align::attr)]
            data-pixi-wrap=[d.wrap.map(num)]
            data-pixi-label=[c.label.as_deref()]
            data-pixi-position=[c.position.map(Vec2::attr)]
            data-pixi-rotation=[c.rotation.map(num)]
            data-pixi-scale=[c.scale.map(Vec2::attr)]
            data-pixi-anchor=[c.anchor.map(Vec2::attr)]
            data-pixi-alpha=[c.alpha.map(num)]
            data-pixi-tint=[c.tint]
            data-pixi-spin=[c.spin.map(num)]
            data-pixi-tap=[c.tap.then_some("true")]
            hx-get=[c.url(Verb::Get)]
            hx-post=[c.url(Verb::Post)]
            hx-trigger=[r.map(|_| "pixi:tap")]
            hx-target=[r.and_then(|r| r.target.as_deref())]
            hx-swap=[r.and_then(|r| r.swap.as_deref())]
            hx-vals=[r.and_then(|r| r.vals.as_deref())] {
            @if let Some(text) = d.text {
                (text)
            }
        }
    }
}

/// An image sprite.
///
/// ```rust
/// use autumn_plugin_pixijs::Sprite;
///
/// let coin = Sprite::new("/static/img/coin.png").position([120.0, 80.0]).spin(90.0);
/// # let _ = coin;
/// ```
///
/// The URL must be `http(s)` or relative. Its path must end with `.png`,
/// `.jpg`, `.jpeg`, `.webp`, `.avif`, or `.svg`. The app CSP must allow it
/// in `connect-src` and `img-src`. The default CSP allows same-origin URLs.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Sprite {
    src: String,
    size: Option<Vec2>,
    common: Common,
}

impl Sprite {
    /// Makes a sprite from an image URL (PNG, JPEG, WebP, AVIF, or SVG).
    pub fn new(src: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            size: None,
            common: Common::default(),
        }
    }

    /// Sets the width and height in logical pixels. Default: the image
    /// size. The method ignores sizes that are not finite and positive.
    /// Sizes clamp to [`MAX_STAGE_SIDE`].
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if let (Some(w), Some(h)) = (positive(width), positive(height)) {
            self.size = Some(Vec2::new(w, h));
        }
        self
    }

    common_setters!();
}

impl Render for Sprite {
    fn render(&self) -> Markup {
        let decl = Decl {
            sprite: Some(&self.src),
            size: self.size,
            ..Decl::default()
        };
        declaration(&decl, &self.common)
    }
}

/// A repeated image that fills a rectangle. It can scroll.
///
/// ```rust
/// use autumn_plugin_pixijs::TilingSprite;
///
/// let sky = TilingSprite::new("/static/img/clouds.png", 800.0, 450.0).scroll(-30.0, 0.0);
/// # let _ = sky;
/// ```
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct TilingSprite {
    src: String,
    size: Option<Vec2>,
    scroll: Option<Vec2>,
    common: Common,
}

impl TilingSprite {
    /// Makes a tiling sprite of `width × height` logical pixels from an
    /// image URL (see [`Sprite`]). The runtime uses the stage size for a size
    /// that is not finite and positive. Sizes clamp to [`MAX_STAGE_SIDE`].
    pub fn new(src: impl Into<String>, width: f32, height: f32) -> Self {
        let size = match (positive(width), positive(height)) {
            (Some(w), Some(h)) => Some(Vec2::new(w, h)),
            _ => None,
        };
        Self {
            src: src.into(),
            size,
            scroll: None,
            common: Common::default(),
        }
    }

    /// Moves the image inside the rectangle, in logical pixels per second.
    /// The scroll stops when the user prefers reduced motion, unless the
    /// stage calls [`Stage::animate_reduced_motion`].
    pub const fn scroll(mut self, x: f32, y: f32) -> Self {
        if let Some(v) = Vec2::new(x, y).finite() {
            self.scroll = Some(v);
        }
        self
    }

    common_setters!();
}

impl Render for TilingSprite {
    fn render(&self) -> Markup {
        let decl = Decl {
            tiling: Some(&self.src),
            size: self.size,
            scroll: self.scroll,
            ..Decl::default()
        };
        declaration(&decl, &self.common)
    }
}

/// A frame animation from a PixiJS sprite sheet (`.json` with its image).
///
/// ```rust
/// use autumn_plugin_pixijs::AnimatedSprite;
///
/// let bird = AnimatedSprite::new("/static/img/bird.json").animation("flap").fps(12.0);
/// # let _ = bird;
/// ```
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct AnimatedSprite {
    src: String,
    animation: Option<String>,
    fps: Option<f32>,
    common: Common,
}

impl AnimatedSprite {
    /// Makes an animated sprite from a sprite sheet URL. The URL path must
    /// end with `.json`, and the sheet must have an `animations` table.
    /// Default: the first animation of the sheet at 12 frames per second.
    pub fn new(src: impl Into<String>) -> Self {
        Self {
            src: src.into(),
            animation: None,
            fps: None,
            common: Common::default(),
        }
    }

    /// Selects an animation of the sheet by name. The method trims the
    /// name. It ignores an empty name. For an unknown name, the runtime
    /// logs a warning and plays the first animation.
    pub fn animation(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        let name = name.trim();
        if !name.is_empty() {
            self.animation = Some(name.to_owned());
        }
        self
    }

    /// Sets the speed in frames per second (`1`–`120`).
    pub const fn fps(mut self, fps: f32) -> Self {
        if let Some(v) = finite(fps) {
            self.fps = Some(v.clamp(FPS.0, FPS.1));
        }
        self
    }

    common_setters!();
}

impl Render for AnimatedSprite {
    fn render(&self) -> Markup {
        let decl = Decl {
            sheet: Some(&self.src),
            animation: self.animation.as_deref(),
            fps: self.fps,
            ..Decl::default()
        };
        declaration(&decl, &self.common)
    }
}

/// A text object. The text is the content of the declaration element, so
/// an htmx swap can change it.
///
/// ```rust
/// use autumn_plugin_pixijs::{Color, Text};
///
/// let score = Text::new("Score: 10").font_size(32.0).fill(Color::WHITE).bold();
/// # let _ = score;
/// ```
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Text {
    content: String,
    font_size: Option<f32>,
    font_family: Option<String>,
    fill: Option<Color>,
    bold: bool,
    align: Option<Align>,
    wrap: Option<f32>,
    common: Common,
}

impl Text {
    /// Makes a text object. Default style: 24 px `sans-serif`, black. The
    /// runtime trims the text and keeps at most 10000 characters.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            content: text.into(),
            font_size: None,
            font_family: None,
            fill: None,
            bold: false,
            align: None,
            wrap: None,
            common: Common::default(),
        }
    }

    /// Sets the font size in logical pixels (`1`–`512`).
    pub const fn font_size(mut self, size: f32) -> Self {
        if let Some(v) = finite(size) {
            self.font_size = Some(v.clamp(FONT_SIZE.0, FONT_SIZE.1));
        }
        self
    }

    /// Sets the CSS font family, for example `"Georgia, serif"`. The runtime
    /// trims it and keeps at most 200 characters.
    pub fn font_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    /// Sets the text color.
    pub const fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }

    /// Uses a bold font weight.
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Sets the alignment of the lines.
    pub const fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    /// Wraps lines at `width` logical pixels. The method ignores a width
    /// that is not finite and positive.
    pub fn wrap(mut self, width: f32) -> Self {
        if let Some(v) = positive(width) {
            self.wrap = Some(v.min(MAX_STAGE_SIDE));
        }
        self
    }

    common_setters!();
}

impl Render for Text {
    fn render(&self) -> Markup {
        let decl = Decl {
            text: Some(&self.content),
            font_size: self.font_size,
            font_family: self.font_family.as_deref(),
            fill: self.fill.map(|c| c.to_string()),
            weight: self.bold.then_some("bold"),
            align: self.align,
            wrap: self.wrap,
            ..Decl::default()
        };
        declaration(&decl, &self.common)
    }
}

/// The geometry of a [`Shape`]. Sizes use logical pixels.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Geometry {
    /// A rectangle (`rect`).
    Rect {
        /// Width.
        width: f32,
        /// Height.
        height: f32,
    },
    /// A rectangle with round corners (`rounded-rect`).
    RoundedRect {
        /// Width.
        width: f32,
        /// Height.
        height: f32,
        /// Corner radius.
        radius: f32,
    },
    /// A circle (`circle`).
    Circle {
        /// Radius.
        radius: f32,
    },
    /// An ellipse (`ellipse`).
    Ellipse {
        /// Horizontal radius.
        radius_x: f32,
        /// Vertical radius.
        radius_y: f32,
    },
    /// A star (`star`).
    Star {
        /// Number of points (`3`–`100`).
        points: u32,
        /// Outer radius.
        outer: f32,
        /// Inner radius.
        inner: f32,
    },
    /// A closed polygon (`polygon`). Corners are relative to the shape
    /// position. The anchor does not apply.
    Polygon(Vec<Vec2>),
}

impl Geometry {
    /// The `data-pixi-shape` value.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Rect { .. } => "rect",
            Self::RoundedRect { .. } => "rounded-rect",
            Self::Circle { .. } => "circle",
            Self::Ellipse { .. } => "ellipse",
            Self::Star { .. } => "star",
            Self::Polygon(_) => "polygon",
        }
    }

    /// The `data-pixi-args` value. A size that is negative or not finite
    /// becomes the default. Sizes clamp to [`MAX_STAGE_SIDE`].
    fn args(&self) -> String {
        let defaults = SHAPE_DEFAULTS
            .iter()
            .find(|(kind, _)| *kind == self.kind())
            .map_or(&[][..], |(_, defaults)| *defaults);
        let sizes = |values: &[f32], defaults: &[f32]| -> Vec<String> {
            values
                .iter()
                .zip(defaults)
                .map(|(v, d)| {
                    num(finite(*v)
                        .filter(|v| *v >= 0.0)
                        .map_or(*d, |v| v.min(MAX_STAGE_SIDE)))
                })
                .collect()
        };
        let list = match self {
            Self::Rect { width, height } => sizes(&[*width, *height], defaults),
            Self::RoundedRect {
                width,
                height,
                radius,
            } => sizes(&[*width, *height, *radius], defaults),
            Self::Circle { radius } => sizes(&[*radius], defaults),
            Self::Ellipse { radius_x, radius_y } => sizes(&[*radius_x, *radius_y], defaults),
            Self::Star {
                points,
                outer,
                inner,
            } => {
                let mut list = vec![(*points).clamp(STAR_POINTS.0, STAR_POINTS.1).to_string()];
                list.extend(sizes(&[*outer, *inner], &defaults[1..]));
                list
            }
            Self::Polygon(points) => {
                let corners: Vec<String> = points
                    .iter()
                    .filter_map(|p| p.finite())
                    .take(MAX_POLYGON_POINTS)
                    .map(Vec2::attr)
                    .collect();
                if corners.len() < 3 {
                    defaults.iter().map(|d| num(*d)).collect()
                } else {
                    corners
                }
            }
        };
        list.join(",")
    }
}

/// The fill of a [`Shape`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fill {
    /// A solid color.
    Color(Color),
    /// No fill (`none`).
    None,
}

impl Fill {
    /// The `data-pixi-fill` value.
    fn attr(self) -> String {
        match self {
            Self::Color(color) => color.to_string(),
            Self::None => "none".to_owned(),
        }
    }
}

/// A vector shape with a fill and an optional stroke.
///
/// ```rust
/// use autumn_plugin_pixijs::{Color, Shape};
///
/// let card = Shape::rounded_rect(200.0, 120.0, 16.0)
///     .fill(Color::hex(0x1e293b))
///     .stroke(Color::hex(0xf59e0b), 4.0);
/// # let _ = card;
/// ```
///
/// The shape centers on its position. [`Shape::anchor`] moves this point
/// within the shape bounds, stroke included. A polygon is different: its
/// corners are relative to the position, and the anchor does not apply.
///
/// A size that is negative or not finite becomes the default of its kind
/// (see the README). A zero size stays zero.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Shape {
    geometry: Geometry,
    fill: Option<Fill>,
    stroke: Option<(Color, Option<f32>)>,
    common: Common,
}

impl Shape {
    /// Makes a shape with a white fill and no stroke.
    pub fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            fill: None,
            stroke: None,
            common: Common::default(),
        }
    }

    /// A rectangle.
    pub fn rect(width: f32, height: f32) -> Self {
        Self::new(Geometry::Rect { width, height })
    }

    /// A rectangle with round corners.
    pub fn rounded_rect(width: f32, height: f32, radius: f32) -> Self {
        Self::new(Geometry::RoundedRect {
            width,
            height,
            radius,
        })
    }

    /// A circle.
    pub fn circle(radius: f32) -> Self {
        Self::new(Geometry::Circle { radius })
    }

    /// An ellipse.
    pub fn ellipse(radius_x: f32, radius_y: f32) -> Self {
        Self::new(Geometry::Ellipse { radius_x, radius_y })
    }

    /// A star with `points` points (clamped to `3`–`100`).
    pub fn star(points: u32, outer: f32, inner: f32) -> Self {
        Self::new(Geometry::Star {
            points,
            outer,
            inner,
        })
    }

    /// A closed polygon. The builder drops corners with a non-finite
    /// component, and corners after [`MAX_POLYGON_POINTS`]. With fewer than
    /// three corners, the shape is the default triangle.
    pub fn polygon<P: Into<Vec2>>(points: impl IntoIterator<Item = P>) -> Self {
        Self::new(Geometry::Polygon(
            points.into_iter().map(Into::into).collect(),
        ))
    }

    /// Sets the fill color. Default: white.
    pub const fn fill(mut self, color: Color) -> Self {
        self.fill = Some(Fill::Color(color));
        self
    }

    /// Removes the fill. Use it with [`Shape::stroke`] for an outline.
    pub const fn no_fill(mut self) -> Self {
        self.fill = Some(Fill::None);
        self
    }

    /// Adds a stroke of `width` logical pixels. A width that is not finite
    /// and positive uses the default (`2`). The width clamps to
    /// [`MAX_STAGE_SIDE`].
    pub fn stroke(mut self, color: Color, width: f32) -> Self {
        self.stroke = Some((color, positive(width)));
        self
    }

    common_setters!();
}

impl Render for Shape {
    fn render(&self) -> Markup {
        let decl = Decl {
            shape: Some(self.geometry.kind()),
            args: Some(self.geometry.args()),
            fill: self.fill.map(Fill::attr),
            stroke: self.stroke.map(|(color, _)| color),
            stroke_width: self.stroke.and_then(|(_, width)| width),
            ..Decl::default()
        };
        declaration(&decl, &self.common)
    }
}

/// One display object of a [`Stage`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StageObject {
    /// An image sprite.
    Sprite(Sprite),
    /// A tiling sprite.
    TilingSprite(TilingSprite),
    /// A sprite-sheet animation.
    AnimatedSprite(AnimatedSprite),
    /// A text object.
    Text(Text),
    /// A vector shape.
    Shape(Shape),
}

macro_rules! stage_object_from {
    ($($ty:ident),*) => {
        $(
            impl From<$ty> for StageObject {
                fn from(object: $ty) -> Self {
                    Self::$ty(object)
                }
            }
        )*
    };
}

stage_object_from!(Sprite, TilingSprite, AnimatedSprite, Text, Shape);

impl StageObject {
    /// True when the object is tappable.
    const fn tappable(&self) -> bool {
        match self {
            Self::Sprite(o) => o.common.tap,
            Self::TilingSprite(o) => o.common.tap,
            Self::AnimatedSprite(o) => o.common.tap,
            Self::Text(o) => o.common.tap,
            Self::Shape(o) => o.common.tap,
        }
    }
}

impl Render for StageObject {
    fn render(&self) -> Markup {
        match self {
            Self::Sprite(o) => o.render(),
            Self::TilingSprite(o) => o.render(),
            Self::AnimatedSprite(o) => o.render(),
            Self::Text(o) => o.render(),
            Self::Shape(o) => o.render(),
        }
    }
}

/// A 2D stage. Renders one `<div data-pixi="stage">` element.
///
/// ```rust
/// use autumn_plugin_pixijs::{Color, Shape, Stage, Text};
/// use autumn_web::html;
///
/// let stage = Stage::new()
///     .label("A spinning orange star")
///     .size(400.0, 300.0)
///     .background(Color::hex(0x0f172a))
///     .add(Shape::star(5, 80.0, 40.0).fill(Color::hex(0xff7a18)).spin(45.0))
///     .add(Text::new("Hello").fill(Color::WHITE).position([200.0, 40.0]))
///     .fallback(html! { p { "The stage needs JavaScript." } });
///
/// let html = html! { (stage) }.into_string();
/// assert!(html.starts_with(r#"<div role="img""#), "{html}");
/// assert!(html.contains(r#"data-pixi="stage""#));
/// assert!(html.contains(r#"data-pixi-shape="star""#));
/// ```
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct Stage {
    id: Option<String>,
    class: Option<String>,
    label: Option<String>,
    size: Option<Vec2>,
    background: Option<Color>,
    renderer: Option<Renderer>,
    animate_reduced_motion: bool,
    objects: Vec<StageObject>,
    fallback: Option<Markup>,
}

impl Stage {
    /// Makes an empty stage with the defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the element `id`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the element `class`, for your own size or layout CSS.
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }

    /// Sets an accessible label. The element gets `aria-label` and
    /// `role="img"`. A stage with tappable objects gets `role="group"`
    /// instead, so that screen readers keep its buttons.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the logical size. Default: `800 × 450`. The runtime sets the
    /// element aspect ratio to `width / height` (clamped to `1/10`–`10`). It
    /// fits the stage in the element and centers it. Sides clamp to `1`–[`MAX_STAGE_SIDE`]. The method
    /// ignores sizes that are not finite and positive.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if let (Some(w), Some(h)) = (positive(width), positive(height)) {
            self.size = Some(Vec2::new(
                w.clamp(1.0, MAX_STAGE_SIDE),
                h.clamp(1.0, MAX_STAGE_SIDE),
            ));
        }
        self
    }

    /// Sets a solid background color. Default: transparent.
    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Sets the renderer. Default: [`Renderer::Auto`].
    pub const fn renderer(mut self, renderer: Renderer) -> Self {
        self.renderer = Some(renderer);
        self
    }

    /// Keeps automatic motion (spin, scroll, animations) when the user
    /// prefers reduced motion. Default: automatic motion stops.
    pub const fn animate_reduced_motion(mut self) -> Self {
        self.animate_reduced_motion = true;
        self
    }

    /// Adds a display object. Later objects draw on top.
    #[expect(
        clippy::should_implement_trait,
        reason = "builder method, not arithmetic"
    )]
    pub fn add(mut self, object: impl Into<StageObject>) -> Self {
        self.objects.push(object.into());
        self
    }

    /// Adds display objects in order. Later objects draw on top.
    pub fn extend<O: Into<StageObject>>(mut self, objects: impl IntoIterator<Item = O>) -> Self {
        self.objects.extend(objects.into_iter().map(Into::into));
        self
    }

    /// Sets fallback content. It shows without JavaScript, without a
    /// renderer, and when an image fails to load.
    pub fn fallback(mut self, markup: Markup) -> Self {
        self.fallback = Some(markup);
        self
    }
}

impl Render for Stage {
    fn render(&self) -> Markup {
        let tappable = self.objects.iter().any(StageObject::tappable);
        let role = if tappable { "group" } else { "img" };
        html! {
            div role=[self.label.as_ref().map(|_| role)]
                aria-label=[self.label.as_deref()]
                id=[self.id.as_deref()]
                class=[self.class.as_deref()]
                data-pixi="stage"
                data-pixi-size=[self.size.map(Vec2::attr)]
                data-pixi-background=[self.background]
                data-pixi-renderer=[self.renderer.map(Renderer::attr)]
                data-pixi-reduced=[self.animate_reduced_motion.then_some("animate")] {
                @for object in &self.objects {
                    (object)
                }
                @if let Some(fallback) = &self.fallback {
                    div data-pixi-fallback { (fallback) }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn render(value: &impl Render) -> String {
        value.render().into_string()
    }

    /// A stage that sets every attribute.
    fn full_stage() -> Stage {
        Stage::new()
            .id("game")
            .class("wide")
            .label("A game")
            .size(640.0, 360.0)
            .background(Color::hex(0x0011_2233))
            .renderer(Renderer::Canvas)
            .animate_reduced_motion()
            .add(
                Sprite::new("/coin.png")
                    .size(32.0, 32.0)
                    .id("coin")
                    .label("Gold coin")
                    .position([10.0, 20.0])
                    .rotation(45.0)
                    .scale([2.0, 3.0])
                    .anchor([0.0, 1.0])
                    .alpha(0.5)
                    .tint(Color::hex(0x00ff_0000))
                    .spin(90.0)
                    .tap_post("/coins/1")
                    .tap_target("#score")
                    .tap_swap("outerHTML")
                    .tap_vals(r#"{"n":1}"#),
            )
            .add(TilingSprite::new("/sky.png", 640.0, 360.0).scroll(-30.0, 5.0))
            .add(AnimatedSprite::new("/bird.json").animation("flap").fps(8.0))
            .add(
                Text::new("Score")
                    .font_size(32.0)
                    .font_family("Georgia, serif")
                    .fill(Color::WHITE)
                    .bold()
                    .align(Align::Center)
                    .wrap(300.0),
            )
            .add(
                Shape::rounded_rect(100.0, 50.0, 8.0)
                    .fill(Color::hex(0x0012_3456))
                    .stroke(Color::BLACK, 3.0),
            )
            .fallback(html! { p { "No canvas." } })
    }

    #[test]
    fn a_stage_with_tappable_objects_is_a_labeled_group() {
        // PixiJS puts its accessible buttons inside the stage. The children
        // of `role="img"` are presentational, so a tappable stage is a group.
        let html = render(
            &Stage::new()
                .label("Game")
                .add(Shape::circle(1.0).tappable()),
        );
        assert!(
            html.starts_with(r#"<div role="group" aria-label="Game""#),
            "{html}"
        );
        let html = render(&Stage::new().label("Art").add(Shape::circle(1.0)));
        assert!(
            html.starts_with(r#"<div role="img" aria-label="Art""#),
            "{html}"
        );
    }

    #[test]
    fn the_last_tap_verb_wins_and_options_need_a_verb() {
        let html = render(&Shape::circle(1.0).tap_get("/a").tap_post("/b"));
        assert!(html.contains(r#"hx-post="/b""#), "{html}");
        assert!(!html.contains("hx-get"), "{html}");
        let html = render(&Shape::circle(1.0).tap_post("/b").tap_get("/a"));
        assert!(html.contains(r#"hx-get="/a""#), "{html}");
        assert!(!html.contains("hx-post"), "{html}");
        let html = render(
            &Shape::circle(1.0)
                .tap_target("#x")
                .tap_swap("outerHTML")
                .tap_vals("{}"),
        );
        assert!(!html.contains("hx-"), "{html}");
        assert!(!html.contains("data-pixi-tap"), "{html}");
    }

    #[test]
    fn sizes_clamp_to_the_largest_stage_side() {
        let big = 1e9;
        let sprite = render(&Sprite::new("/a.png").size(big, 1.0));
        assert!(sprite.contains(r#"data-pixi-size="8192,1""#), "{sprite}");
        let tiling = render(&TilingSprite::new("/a.png", big, 2.0));
        assert!(tiling.contains(r#"data-pixi-size="8192,2""#), "{tiling}");
        let stroke = render(&Shape::circle(1.0).stroke(Color::BLACK, big));
        assert!(
            stroke.contains(r#"data-pixi-stroke-width="8192""#),
            "{stroke}"
        );
        assert_eq!(Shape::rect(big, 1.0).geometry.args(), "8192,1");
        assert_eq!(Shape::star(5, big, big).geometry.args(), "5,8192,8192");
    }

    /// Defaults that the README and rustdoc name. `parse.js` must agree.
    const DOCUMENTED_DEFAULTS: [&str; 4] = [
        "fontSize: 24",
        "strokeWidth: 2",
        "fps: 12",
        "maxText: 10000",
    ];

    #[test]
    fn limits_and_defaults_match_parse_js() {
        let parse = include_str!("../assets/parse.js");
        let mut needles = vec![
            format!("maxSide: {MAX_STAGE_SIDE}"),
            format!("maxPolygonPoints: {MAX_POLYGON_POINTS}"),
            format!(
                "starPoints: Object.freeze([{}, {}])",
                STAR_POINTS.0, STAR_POINTS.1
            ),
            format!(
                "fontSize: Object.freeze([{}, {}])",
                FONT_SIZE.0, FONT_SIZE.1
            ),
            format!("fps: Object.freeze([{}, {}])", FPS.0, FPS.1),
            format!(
                "size: Object.freeze([{}, {}])",
                DEFAULT_STAGE_SIZE.x, DEFAULT_STAGE_SIZE.y
            ),
        ];
        needles.extend(DOCUMENTED_DEFAULTS.map(str::to_owned));
        for (kind, defaults) in SHAPE_DEFAULTS {
            let list: Vec<String> = defaults.iter().map(|d| num(*d)).collect();
            needles.push(format!(r#""{kind}": Object.freeze([{}])"#, list.join(", ")));
        }
        for needle in needles {
            assert!(parse.contains(&needle), "parse.js has {needle}");
        }
        let kinds: Vec<&str> = all_geometries().iter().map(Geometry::kind).collect();
        assert_eq!(kinds, SHAPE_DEFAULTS.map(|(kind, _)| kind));
    }

    #[test]
    fn number_grammar_matches_parse_js() {
        // `js_number` copies this regex. Keep both in sync.
        let parse = include_str!("../assets/parse.js");
        assert!(parse.contains(r"const NUMBER = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i;"));
    }

    #[test]
    fn small_conveniences_work() {
        assert_eq!(Vec2::from(2.0), Vec2::splat(2.0));
        assert_eq!(Align::default(), Align::Left);
        let one = Stage::new().add(Shape::circle(1.0)).add(Text::new("a"));
        let both: Vec<StageObject> = vec![Shape::circle(1.0).into(), Text::new("a").into()];
        assert_eq!(render(&Stage::new().extend(both)), render(&one));
    }

    #[test]
    fn empty_stage_is_one_bare_element() {
        assert_eq!(render(&Stage::new()), r#"<div data-pixi="stage"></div>"#);
    }

    #[test]
    fn full_stage_renders_every_stage_attribute() {
        let html = render(&full_stage());
        let open = &html[..html.find('>').expect("tag")];
        for attr in [
            r#"role="group""#,
            r#"aria-label="A game""#,
            r#"id="game""#,
            r#"class="wide""#,
            r#"data-pixi="stage""#,
            r#"data-pixi-size="640,360""#,
            r##"data-pixi-background="#112233""##,
            r#"data-pixi-renderer="canvas""#,
            r#"data-pixi-reduced="animate""#,
        ] {
            assert!(open.contains(attr), "{attr}: {open}");
        }
        assert!(html.ends_with(r"<div data-pixi-fallback><p>No canvas.</p></div></div>"));
    }

    #[test]
    fn sprite_renders_every_common_attribute() {
        let html = render(&full_stage().objects[0]);
        assert!(html.starts_with("<div hidden "), "{html}");
        for attr in [
            r#"id="coin""#,
            r#"data-pixi-sprite="/coin.png""#,
            r#"data-pixi-size="32,32""#,
            r#"data-pixi-label="Gold coin""#,
            r#"data-pixi-position="10,20""#,
            r#"data-pixi-rotation="45""#,
            r#"data-pixi-scale="2,3""#,
            r#"data-pixi-anchor="0,1""#,
            r#"data-pixi-alpha="0.5""#,
            r##"data-pixi-tint="#ff0000""##,
            r#"data-pixi-spin="90""#,
            r#"data-pixi-tap="true""#,
            r#"hx-post="/coins/1""#,
            r#"hx-trigger="pixi:tap""#,
            r##"hx-target="#score""##,
            r#"hx-swap="outerHTML""#,
            r#"hx-vals="{&quot;n&quot;:1}""#,
        ] {
            assert!(html.contains(attr), "{attr}: {html}");
        }
        assert!(html.ends_with("></div>"), "{html}");
    }

    #[test]
    fn other_objects_render_their_attributes() {
        let stage = full_stage();
        let tiling = render(&stage.objects[1]);
        for attr in [
            r#"data-pixi-tiling="/sky.png""#,
            r#"data-pixi-size="640,360""#,
            r#"data-pixi-scroll="-30,5""#,
        ] {
            assert!(tiling.contains(attr), "{attr}: {tiling}");
        }
        let sheet = render(&stage.objects[2]);
        for attr in [
            r#"data-pixi-sheet="/bird.json""#,
            r#"data-pixi-animation="flap""#,
            r#"data-pixi-fps="8""#,
        ] {
            assert!(sheet.contains(attr), "{attr}: {sheet}");
        }
        let text = render(&stage.objects[3]);
        for attr in [
            r#"data-pixi-text="""#,
            r#"data-pixi-font-size="32""#,
            r#"data-pixi-font-family="Georgia, serif""#,
            r##"data-pixi-fill="#ffffff""##,
            r#"data-pixi-weight="bold""#,
            r#"data-pixi-align="center""#,
            r#"data-pixi-wrap="300""#,
        ] {
            assert!(text.contains(attr), "{attr}: {text}");
        }
        assert!(text.ends_with(">Score</div>"), "{text}");
        let shape = render(&stage.objects[4]);
        for attr in [
            r#"data-pixi-shape="rounded-rect""#,
            r#"data-pixi-args="100,50,8""#,
            r##"data-pixi-fill="#123456""##,
            r##"data-pixi-stroke="#000000""##,
            r#"data-pixi-stroke-width="3""#,
        ] {
            assert!(shape.contains(attr), "{attr}: {shape}");
        }
    }

    #[test]
    fn unset_options_render_no_attributes() {
        let html = render(&Sprite::new("/a.png"));
        assert_eq!(html, r#"<div hidden data-pixi-sprite="/a.png"></div>"#);
        let html = render(&Shape::circle(10.0));
        assert_eq!(
            html,
            r#"<div hidden data-pixi-shape="circle" data-pixi-args="10"></div>"#
        );
    }

    #[test]
    fn tappable_without_request_has_no_htmx_attributes() {
        let html = render(&Shape::circle(10.0).tappable());
        assert!(html.contains(r#"data-pixi-tap="true""#), "{html}");
        assert!(!html.contains("hx-"), "{html}");
        let html = render(&Shape::circle(10.0).tap_get("/x"));
        assert!(html.contains(r#"hx-get="/x""#), "{html}");
        assert!(html.contains(r#"hx-trigger="pixi:tap""#), "{html}");
    }

    #[test]
    fn objects_render_in_order_before_the_fallback() {
        let html = render(&full_stage());
        let at = |needle: &str| html.find(needle).expect(needle);
        assert!(at("data-pixi-sprite") < at("data-pixi-tiling"));
        assert!(at("data-pixi-tiling") < at("data-pixi-sheet"));
        assert!(at("data-pixi-sheet") < at("data-pixi-text"));
        assert!(at("data-pixi-text") < at("data-pixi-shape"));
        assert!(at("data-pixi-shape") < at("data-pixi-fallback"));
    }

    #[test]
    fn every_shape_renders_its_kind_and_sizes() {
        let cases = [
            (Shape::rect(10.0, 20.0), "rect", "10,20"),
            (
                Shape::rounded_rect(10.0, 20.0, 3.0),
                "rounded-rect",
                "10,20,3",
            ),
            (Shape::circle(5.0), "circle", "5"),
            (Shape::ellipse(6.0, 4.0), "ellipse", "6,4"),
            (Shape::star(6, 50.0, 20.0), "star", "6,50,20"),
            (
                Shape::polygon([[0.0, 0.0], [10.0, 0.0], [5.0, 8.5]]),
                "polygon",
                "0,0,10,0,5,8.5",
            ),
        ];
        for (shape, kind, args) in cases {
            let html = render(&shape);
            assert!(
                html.contains(&format!(r#"data-pixi-shape="{kind}""#)),
                "{html}"
            );
            assert!(
                html.contains(&format!(r#"data-pixi-args="{args}""#)),
                "{html}"
            );
        }
    }

    #[test]
    fn bad_sizes_render_as_defaults() {
        let args = |shape: Shape| shape.geometry.args();
        assert_eq!(args(Shape::rect(f32::NAN, -1.0)), "100,100");
        assert_eq!(args(Shape::rounded_rect(1.0, 2.0, f32::INFINITY)), "1,2,12");
        assert_eq!(args(Shape::circle(-5.0)), "50");
        assert_eq!(args(Shape::ellipse(f32::NAN, 3.0)), "60,3");
        assert_eq!(args(Shape::star(1, f32::NAN, -2.0)), "3,50,25");
        assert_eq!(args(Shape::star(1000, 10.0, 5.0)), "100,10,5");
        assert_eq!(
            args(Shape::polygon([[0.0, 0.0], [f32::NAN, 1.0]])),
            "0,-50,50,50,-50,50"
        );
    }

    #[test]
    fn polygons_keep_finite_corners_up_to_the_limit() {
        let many = (0..1000_u16).map(|i| [f32::from(i), 0.0]);
        let args = Shape::polygon(many).geometry.args();
        assert_eq!(args.split(',').count(), MAX_POLYGON_POINTS * 2);
        let some = Shape::polygon([[0.0, 0.0], [f32::INFINITY, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        assert_eq!(some.geometry.args(), "0,0,1,0,1,1");
    }

    #[test]
    fn setters_ignore_non_finite_input() {
        let nan = f32::NAN;
        let sprite = Sprite::new("/a.png")
            .size(nan, 1.0)
            .position([nan, 1.0])
            .rotation(nan)
            .scale([1.0, f32::INFINITY])
            .anchor([nan, nan])
            .alpha(nan)
            .spin(f32::NEG_INFINITY);
        assert_eq!(render(&sprite), render(&Sprite::new("/a.png")));
        let stage = Stage::new().size(nan, 100.0).size(0.0, 100.0);
        assert_eq!(render(&stage), render(&Stage::new()));
        let text = Text::new("a").font_size(nan).wrap(nan).wrap(-1.0);
        assert_eq!(render(&text), render(&Text::new("a")));
        let sheet = AnimatedSprite::new("/a.json").fps(nan);
        assert_eq!(render(&sheet), render(&AnimatedSprite::new("/a.json")));
        let tiling = TilingSprite::new("/a.png", nan, 1.0).scroll(nan, 1.0);
        assert!(!render(&tiling).contains("size"), "{}", render(&tiling));
        assert!(!render(&tiling).contains("scroll"), "{}", render(&tiling));
        let shape = Shape::circle(1.0).stroke(Color::BLACK, nan);
        assert!(!render(&shape).contains("stroke-width"));
        assert!(render(&shape).contains("data-pixi-stroke="));
    }

    #[test]
    fn ranges_are_clamped() {
        assert_eq!(Sprite::new("/a").alpha(2.0).common.alpha, Some(1.0));
        assert_eq!(Sprite::new("/a").alpha(-2.0).common.alpha, Some(0.0));
        assert_eq!(Text::new("a").font_size(0.0).font_size, Some(1.0));
        assert_eq!(Text::new("a").font_size(9999.0).font_size, Some(512.0));
        assert_eq!(Text::new("a").wrap(1e9).wrap, Some(MAX_STAGE_SIDE));
        assert_eq!(AnimatedSprite::new("/a").fps(0.0).fps, Some(1.0));
        assert_eq!(AnimatedSprite::new("/a").fps(500.0).fps, Some(120.0));
        assert_eq!(
            Stage::new().size(1e9, 0.5).size,
            Some(Vec2::new(MAX_STAGE_SIDE, 1.0))
        );
    }

    #[test]
    fn fill_and_no_fill_render_their_values() {
        assert!(render(&Shape::circle(1.0).no_fill()).contains(r#"data-pixi-fill="none""#));
        assert!(!render(&Shape::circle(1.0)).contains("data-pixi-fill"));
    }

    #[test]
    fn animation_names_are_trimmed_and_empty_names_ignored() {
        let sheet = AnimatedSprite::new("/a.json").animation("  run  ");
        assert_eq!(sheet.animation.as_deref(), Some("run"));
        let sheet = AnimatedSprite::new("/a.json").animation("   ");
        assert_eq!(sheet.animation, None);
    }

    #[test]
    fn enums_render_their_names() {
        assert_eq!(
            ALL_RENDERERS.map(Renderer::attr),
            ["auto", "webgl", "canvas"]
        );
        assert_eq!(ALL_ALIGNS.map(Align::attr), ["left", "center", "right"]);
        assert_eq!(Renderer::default(), Renderer::Auto);
    }

    #[test]
    fn defaults_match_the_documented_defaults() {
        let parse = include_str!("../assets/parse.js");
        assert!(
            parse.contains("size: Object.freeze([800, 450])"),
            "stage size"
        );
        assert_eq!(DEFAULT_STAGE_SIZE, Vec2::new(800.0, 450.0));
        assert!(parse.contains(&format!("maxSide: {MAX_STAGE_SIDE}")));
        assert!(parse.contains(&format!("maxPolygonPoints: {MAX_POLYGON_POINTS}")));
    }

    #[test]
    fn colors_format_as_hex() {
        assert_eq!(Color::BLACK.to_string(), "#000000");
        assert_eq!(Color::WHITE.to_string(), "#ffffff");
        assert_eq!(Color::hex(0xff12_3456).value(), 0x0012_3456);
        assert_eq!(Color::rgb(1, 2, 3).to_string(), "#010203");
    }

    #[test]
    fn vec2_conversions_agree() {
        assert_eq!(Vec2::from((1.0, 2.0)), Vec2::new(1.0, 2.0));
        assert_eq!(Vec2::from([1.0, 2.0]), Vec2::new(1.0, 2.0));
        assert_eq!(Vec2::ZERO, Vec2::splat(0.0));
    }

    #[test]
    fn user_text_is_escaped() {
        let html = render(
            &Stage::new()
                .label(r#""><script>alert(1)</script>"#)
                .add(Sprite::new(r#"/a.png"><script>"#).tap_get(r#"/x"y"#))
                .add(Text::new("<b>bold</b> & co")),
        );
        assert!(!html.contains("<script>"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
        assert!(html.contains("&quot;&gt;&lt;script&gt;"), "{html}");
        assert!(html.contains("&lt;b&gt;bold&lt;/b&gt; &amp; co"), "{html}");
    }

    #[test]
    fn stage_renders_inside_maud() {
        let html = html! { main { (Stage::new()) } }.into_string();
        assert_eq!(html, r#"<main><div data-pixi="stage"></div></main>"#);
    }

    #[test]
    fn every_emitted_attribute_and_value_is_known_to_the_runtime() {
        // Rust and JS stay in lockstep: parse.js must name every attribute
        // and every keyword value the builder emits.
        let parse = include_str!("../assets/parse.js");
        let html = render(&full_stage().add(Shape::circle(1.0).no_fill()));
        let mut names: Vec<&str> = html
            .split(|c: char| c.is_whitespace() || c == '<' || c == '>')
            .filter_map(|token| token.split('=').next())
            .filter(|name| name.starts_with("data-pixi"))
            .collect();
        names.sort_unstable();
        names.dedup();
        assert!(names.len() >= 30, "{names:?}");
        for name in names {
            assert!(
                parse.contains(&format!("\"{name}\"")),
                "parse.js names {name}"
            );
        }
        let mut keywords: Vec<&str> = all_geometries().iter().map(Geometry::kind).collect();
        keywords.extend(ALL_RENDERERS.map(Renderer::attr));
        keywords.extend(ALL_ALIGNS.map(Align::attr));
        keywords.extend(["animate", "bold", "none", "true", "stage"]);
        for value in keywords {
            assert!(
                parse.contains(&format!("\"{value}\"")),
                "parse.js knows {value}"
            );
        }
    }

    /// Every `Renderer` variant. The match fails to compile when a variant
    /// is added.
    const ALL_RENDERERS: [Renderer; 3] = [Renderer::Auto, Renderer::WebGl, Renderer::Canvas];
    const _: fn(Renderer) = |r| match r {
        Renderer::Auto | Renderer::WebGl | Renderer::Canvas => {}
    };
    /// Every `Align` variant.
    const ALL_ALIGNS: [Align; 3] = [Align::Left, Align::Center, Align::Right];
    const _: fn(Align) = |a| match a {
        Align::Left | Align::Center | Align::Right => {}
    };

    /// One value of every `Geometry` variant.
    fn all_geometries() -> Vec<Geometry> {
        let geometries: Vec<Geometry> = [
            Shape::rect(1.0, 1.0),
            Shape::rounded_rect(1.0, 1.0, 1.0),
            Shape::circle(1.0),
            Shape::ellipse(1.0, 1.0),
            Shape::star(5, 1.0, 0.5),
            Shape::polygon([[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
        ]
        .map(|shape| shape.geometry)
        .into();
        for g in &geometries {
            // Fails to compile when a variant is added.
            match g {
                Geometry::Rect { .. }
                | Geometry::RoundedRect { .. }
                | Geometry::Circle { .. }
                | Geometry::Ellipse { .. }
                | Geometry::Star { .. }
                | Geometry::Polygon(_) => {}
            }
        }
        geometries
    }

    /// True when `token` matches the parse.js `NUMBER` grammar:
    /// `^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$` (any case).
    fn js_number(token: &str) -> bool {
        let body = token.strip_prefix(['+', '-']).unwrap_or(token);
        let (mantissa, exponent) = match body.split_once(['e', 'E']) {
            Some((m, e)) => (m, Some(e)),
            None => (body, None),
        };
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        let mantissa_ok = match mantissa.split_once('.') {
            Some((int, frac)) => {
                (digits(int) && (frac.is_empty() || digits(frac)))
                    || (int.is_empty() && digits(frac))
            }
            None => digits(mantissa),
        };
        let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
        mantissa_ok && exponent_ok
    }

    #[test]
    fn js_number_matches_the_parse_js_grammar() {
        for good in ["0", "-1", "+2.5", "3.", ".5", "1e5", "1E-5"] {
            assert!(js_number(good), "{good}");
        }
        for bad in [
            "", "-", ".", "1.2.3", "NaN", "inf", "1e", "0x10", "1px", "e5",
        ] {
            assert!(!js_number(bad), "{bad}");
        }
    }

    /// Any `f32`, with non-finite values made likely.
    fn any_f32() -> impl Strategy<Value = f32> {
        prop_oneof![
            Just(f32::NAN),
            Just(f32::INFINITY),
            Just(f32::NEG_INFINITY),
            Just(-0.0_f32),
            any::<f32>(),
        ]
    }

    /// Each number inside the numeric `data-pixi-*` attribute values.
    fn numbers(html: &str) -> Vec<String> {
        let mut out = Vec::new();
        for part in html.split("data-pixi-").skip(1) {
            let Some(value) = part.split('"').nth(1) else {
                continue;
            };
            if value.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
                out.extend(value.split(',').map(str::to_owned));
            }
        }
        out
    }

    proptest! {
        #[test]
        fn builder_never_emits_non_finite_numbers(
            a in any_f32(), b in any_f32(), c in any_f32(), d in any_f32(), n in any::<u32>()
        ) {
            let html = render(
                &Stage::new()
                    .size(a, b)
                    .add(
                        Sprite::new("/a.png")
                            .size(c, d)
                            .position([a, b])
                            .rotation(c)
                            .scale([d, a])
                            .anchor([b, c])
                            .alpha(d)
                            .spin(a),
                    )
                    .add(TilingSprite::new("/t.png", a, b).scroll(c, d))
                    .add(AnimatedSprite::new("/s.json").fps(a))
                    .add(Text::new("t").font_size(b).wrap(c))
                    .add(Shape::rect(a, b).stroke(Color::BLACK, c).position([1.0, 2.0]))
                    .add(Shape::rounded_rect(c, d, a))
                    .add(Shape::circle(b))
                    .add(Shape::ellipse(c, d))
                    .add(Shape::star(n, a, b))
                    .add(Shape::polygon([[a, b], [c, d], [b, a], [d, c]])),
            );
            prop_assert!(!html.contains("NaN"), "{}", html);
            prop_assert!(!html.contains("inf"), "{}", html);
            let numbers = numbers(&html);
            prop_assert!(numbers.len() >= 15, "{:?}", numbers);
            for number in numbers {
                prop_assert!(js_number(&number), "parse.js reads {}", number);
                let parsed: f32 = number.parse().map_err(|e| TestCaseError::fail(format!("{number}: {e}")))?;
                prop_assert!(parsed.is_finite(), "{}", number);
                prop_assert!(number != "-0", "{}", html);
            }
        }

        #[test]
        fn vec2_attr_round_trips_finite_values(x in any::<f32>(), y in any::<f32>()) {
            prop_assume!(x.is_finite() && y.is_finite());
            let attr = Vec2::new(x, y).attr();
            let parts: Vec<f32> = attr.split(',').map(|p| p.parse().expect("number")).collect();
            prop_assert_eq!(parts, vec![x, y]);
            prop_assert!(attr.split(',').all(js_number), "{}", attr);
        }

        #[test]
        fn clamped_values_stay_in_range(v in any_f32(), w in any_f32(), n in any::<u32>()) {
            if let Some(alpha) = Sprite::new("/a").alpha(v).common.alpha {
                prop_assert!((0.0..=1.0).contains(&alpha));
            }
            if let Some(size) = Text::new("a").font_size(v).font_size {
                prop_assert!((1.0..=512.0).contains(&size));
            }
            if let Some(fps) = AnimatedSprite::new("/a").fps(v).fps {
                prop_assert!((1.0..=120.0).contains(&fps));
            }
            if let Some(size) = Stage::new().size(v, w).size {
                prop_assert!((1.0..=MAX_STAGE_SIDE).contains(&size.x));
                prop_assert!((1.0..=MAX_STAGE_SIDE).contains(&size.y));
            }
            let star = Shape::star(n, v, w).geometry.args();
            let points: u32 = star.split(',').next().and_then(|p| p.parse().ok()).unwrap_or(0);
            prop_assert!((3..=100).contains(&points), "{}", star);
        }
    }
}
