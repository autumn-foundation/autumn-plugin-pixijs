//! This crate adds PixiJS 2D canvas stages to Autumn apps.

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
