//! [`PixiPlugin`]: installs the PixiJS assets in an Autumn app.
//!
//! The plugin installs [`PIXI_ASSETS`] through `AppBuilder::plugin_assets`.
//! It reads no configuration and adds no startup hooks.

use std::borrow::Cow;

use autumn_web::app::AppBuilder;
use autumn_web::plugin::Plugin;

use crate::assets::PIXI_ASSETS;

/// The plugin name in Autumn diagnostics.
pub const PLUGIN_NAME: &str = "autumn-plugin-pixijs";

/// Installs the PixiJS assets in an Autumn app.
///
/// ```rust,no_run
/// use autumn_plugin_pixijs::PixiPlugin;
///
/// # async fn run() {
/// autumn_web::app()
///     .plugin(PixiPlugin::new())
///     .run()
///     .await;
/// # }
/// ```
///
/// Then put [`pixi_script`](crate::pixi_script) and
/// [`pixi_stylesheet`](crate::pixi_stylesheet) in the page `<head>`.
#[derive(Debug, Default)]
#[must_use]
pub struct PixiPlugin;

impl PixiPlugin {
    /// Makes the plugin. It reads no configuration.
    pub const fn new() -> Self {
        Self
    }
}

impl Plugin for PixiPlugin {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(PLUGIN_NAME)
    }

    fn build(self, app: AppBuilder) -> AppBuilder {
        app.plugin_assets(&PIXI_ASSETS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{INIT_JS, PARSE_JS, PIXI_ASSETS, PIXI_CSS, PIXI_JS, UNSAFE_EVAL_JS};
    use autumn_web::assets::{PLUGIN_ASSETS_ROUTE_MARKER, asset_url};
    use autumn_web::plugin_conformance::{ConformanceConfig, run_conformance};
    use autumn_web::route_listing::{RouteClassification, RouteSource};
    use autumn_web::test::{TestApp, TestClient};

    const JS: &str = "text/javascript; charset=utf-8";
    const CSS: &str = "text/css; charset=utf-8";
    const IMMUTABLE: &str = "public, max-age=31536000, immutable";
    const REVALIDATE: &str = "public, max-age=0, must-revalidate";
    const ALL: [&str; 5] = [PIXI_JS, UNSAFE_EVAL_JS, PARSE_JS, INIT_JS, PIXI_CSS];

    fn client() -> TestClient {
        TestApp::new().plugin(PixiPlugin::new()).build()
    }

    #[tokio::test]
    async fn every_file_serves_at_its_fingerprinted_url() {
        let client = client();
        for path in ALL {
            let asset = PIXI_ASSETS.get(path).expect("bundled");
            let response = client.get(asset.url()).send().await;
            let expected = if path == PIXI_CSS { CSS } else { JS };
            response
                .assert_ok()
                .assert_header("content-type", expected)
                .assert_header("cache-control", IMMUTABLE);
            assert_eq!(response.body.as_slice(), asset.bytes(), "{path}");
        }
    }

    #[tokio::test]
    async fn files_serve_at_plain_urls_with_etags() {
        // `init.js` imports `./parse.js` at its plain URL. It must work.
        let client = client();
        for path in ALL {
            let plain = format!("/static/_plugins/pixi/{path}");
            let response = client.get(&plain).send().await;
            response
                .assert_ok()
                .assert_header("cache-control", REVALIDATE);
            let etag = response.header("etag").expect("etag").to_owned();
            client
                .get(&plain)
                .header("if-none-match", &etag)
                .send()
                .await
                .assert_status(304);
        }
    }

    #[tokio::test]
    async fn unbundled_and_stale_paths_are_not_found() {
        let client = client();
        for path in [
            "/static/_plugins/pixi/manifest.json",
            "/static/_plugins/pixi/PIXI-LICENSE",
            "/static/_plugins/pixi/init.00000000.js",
            "/static/_plugins/pixi/pixi.min.js.map",
            "/static/_plugins/pixi/pixi.min.mjs",
        ] {
            client.get(path).send().await.assert_status(404);
        }
    }

    #[tokio::test]
    async fn asset_url_resolves_the_installed_bundle() {
        let _client = client();
        for path in ALL {
            assert_eq!(
                asset_url(&format!("_plugins/pixi/{path}")),
                PIXI_ASSETS.url(path)
            );
        }
    }

    #[test]
    fn bundle_routes_are_public_plugin_routes() {
        let app = autumn_web::app().plugin(PixiPlugin::new());
        let infos = app.plugin_route_infos().expect("route infos");
        let routes: Vec<_> = infos
            .iter()
            .filter(|info| info.path.starts_with("/static/_plugins/pixi/"))
            .collect();
        assert_eq!(routes.len(), 10, "five files, two URLs each: {infos:?}");
        for info in routes {
            assert_eq!(info.method, "GET");
            assert_eq!(info.classification, RouteClassification::Public);
            assert_eq!(info.middleware, [PLUGIN_ASSETS_ROUTE_MARKER]);
            assert_eq!(info.source, RouteSource::Plugin(PLUGIN_NAME.to_owned()));
        }
    }

    #[test]
    fn plugin_passes_conformance() {
        let app = autumn_web::app().plugin(PixiPlugin::new());
        let infos = app.plugin_route_infos().expect("route infos");
        let report = run_conformance(&ConformanceConfig::new(PLUGIN_NAME), &infos);
        assert!(report.passed(), "{}", report.to_text_report());
    }

    #[tokio::test]
    async fn installing_the_plugin_twice_is_harmless() {
        let client = TestApp::new()
            .plugin(PixiPlugin::new())
            .plugin(PixiPlugin::new())
            .build();
        client
            .get(&PIXI_ASSETS.url(INIT_JS))
            .send()
            .await
            .assert_ok();
    }

    #[test]
    fn plugin_name_is_the_crate_name() {
        assert_eq!(PixiPlugin::new().name(), env!("CARGO_PKG_NAME"));
    }
}
