//! This crate adds PixiJS 2D canvas stages to Autumn apps. It works with
//! Maud and htmx.
//!
//! Add [`PixiPlugin`] to the app. Put [`pixi_stylesheet()`] and
//! [`pixi_script()`] in the page `<head>`. Then write a [`Stage`]:
//!
//! ```rust,no_run
//! use autumn_plugin_pixijs::{
//!     Color, PixiPlugin, Shape, Sprite, Stage, Text, pixi_script, pixi_stylesheet,
//! };
//! use autumn_web::prelude::*;
//!
//! #[get("/")]
//! async fn index() -> Markup {
//!     html! {
//!         html {
//!             head { (pixi_stylesheet()) (pixi_script()) }
//!             body {
//!                 (Stage::new()
//!                     .id("game")
//!                     .label("A coin game")
//!                     .background(Color::hex(0x0f172a))
//!                     .add(Text::new("Tap the coin").fill(Color::WHITE).position([400.0, 60.0]))
//!                     .add(Shape::star(5, 120.0, 50.0).fill(Color::hex(0xff7a18)).spin(30.0))
//!                     .add(Sprite::new("/static/img/coin.png")
//!                         .label("Gold coin")
//!                         .tap_post("/collect")
//!                         .tap_target("#game")
//!                         .tap_swap("outerHTML")))
//!             }
//!         }
//!     }
//! }
//!
//! # async fn run() {
//! autumn_web::app()
//!     .plugin(PixiPlugin::new())
//!     .routes(routes![index])
//!     .run()
//!     .await;
//! # }
//! ```
//!
//! # How it works
//!
//! - The crate contains [PixiJS](https://pixijs.com) 8.22.0 (MIT) and its
//!   `unsafe-eval` package. It does not use npm or a bundler.
//!   [`PIXI_ASSETS`] serves them under `/static/_plugins/pixi/`. SRI covers
//!   every tag.
//! - The builder renders `data-pixi-*` attributes. `init.js` reads them and
//!   builds the stage. You can also write the attributes by hand.
//! - `init.js` scans on load, on `htmx:afterSwap`, and on each DOM insertion.
//!   It builds a stage again when its declarations change. It frees a stage
//!   when its element leaves the document.
//! - A tappable object sends a `pixi:tap` event from its declaration. An
//!   htmx trigger can use it.
//! - The `pixi:ready` event gives your own JavaScript the application, the
//!   root container, and the global `PIXI`.
//!
//! # Limits
//!
//! - The plugin uses WebGL or Canvas 2D. It does not use the WebGPU
//!   renderer.
//! - The declarative layer has no filters, masks, particles, or physics.
//!   Use the `pixi:ready` event for custom code.
//! - Each WebGL stage has its own context. Browsers keep about 16 contexts.
//!   [`Renderer::Canvas`] uses no WebGL context.
//! - Do not let user content keep `data-pixi-*` or `hx-*` attributes. User
//!   markup could then start stages, load image URLs, and send requests.

mod assets;
mod plugin;
mod script;
mod stage;

pub use assets::{ASSETS_NAMESPACE, PIXI_ASSETS, PIXI_VERSION, VENDORED, VendoredFile};
pub use plugin::{PLUGIN_NAME, PixiPlugin};
pub use script::{pixi_script, pixi_stylesheet};
pub use stage::{
    Align, AnimatedSprite, Color, DEFAULT_STAGE_SIZE, Geometry, MAX_POLYGON_POINTS, MAX_STAGE_SIDE,
    Renderer, Shape, Sprite, Stage, StageObject, Text, TilingSprite, Vec2,
};
