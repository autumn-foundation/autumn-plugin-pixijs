//! This crate adds PixiJS 2D canvas stages to Autumn apps.

mod assets;
mod plugin;
mod script;

pub use assets::{ASSETS_NAMESPACE, PIXI_ASSETS, PIXI_VERSION, VENDORED, VendoredFile};
pub use plugin::{PLUGIN_NAME, PixiPlugin};
pub use script::{pixi_script, pixi_stylesheet};
