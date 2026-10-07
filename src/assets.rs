//! Vendored PixiJS assets, embedded at compile time.
//!
//! The bundle holds:
//!
//! - PixiJS: `pixi.min.js` (IIFE build, global `PIXI`).
//! - The PixiJS `unsafe-eval` package: `unsafe-eval.min.js`. It patches the
//!   global `PIXI`, so that PixiJS needs no `eval`.
//! - Plugin files: `parse.js`, `init.js`, `pixi.css`.
//!
//! The vendored files are upstream bytes. The plugin does not change them.
//!
//! [`PixiPlugin`](crate::PixiPlugin) installs [`PIXI_ASSETS`] through
//! `AppBuilder::plugin_assets`. Autumn serves each file under
//! `/static/_plugins/pixi/` at a hashed URL (immutable) and at its plain
//! URL (`must-revalidate`). Autumn computes the SRI hashes.
//!
//! `manifest.json` and `PIXI-LICENSE` are not in the bundle. They are not
//! served.

use autumn_web::assets::PluginAssets;

/// URL namespace of the bundle. Files are served under
/// `/static/_plugins/pixi/`.
pub const ASSETS_NAMESPACE: &str = "pixi";

/// Pinned PixiJS version.
pub const PIXI_VERSION: &str = "8.22.0";

/// PixiJS IIFE build. It sets the global `PIXI`.
pub(crate) const PIXI_JS: &str = "pixi.min.js";
/// PixiJS `unsafe-eval` package. It patches the global `PIXI`.
pub(crate) const UNSAFE_EVAL_JS: &str = "unsafe-eval.min.js";
/// Plugin attribute parsers (no PixiJS dependency).
pub(crate) const PARSE_JS: &str = "parse.js";
/// Plugin runtime: scans `[data-pixi="stage"]` and builds stages.
pub(crate) const INIT_JS: &str = "init.js";
/// Plugin default styles.
pub(crate) const PIXI_CSS: &str = "pixi.css";

/// The plugin asset bundle.
///
/// [`PixiPlugin`](crate::PixiPlugin) installs it. Use it directly only to
/// make URLs or tags yourself:
///
/// ```rust
/// use autumn_plugin_pixijs::PIXI_ASSETS;
///
/// let url = PIXI_ASSETS.url("init.js");
/// assert!(url.starts_with("/static/_plugins/pixi/init."), "{url}");
/// let sri = PIXI_ASSETS.integrity("init.js").expect("init.js is bundled");
/// assert!(sri.starts_with("sha384-"));
/// ```
pub static PIXI_ASSETS: PluginAssets = PluginAssets::from_files(ASSETS_NAMESPACE, &[]);

/// Provenance of one vendored upstream file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct VendoredFile {
    /// Logical path in [`PIXI_ASSETS`].
    pub path: &'static str,
    /// Upstream URL (jsDelivr, pinned version).
    pub source: &'static str,
    /// `sha384` SRI of the upstream bytes. The bundled bytes are the same.
    pub upstream_integrity: &'static str,
}

/// The vendored upstream files and their pins.
pub const VENDORED: &[VendoredFile] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use sha2::{Digest as _, Sha384};

    /// Computes the `sha384` SRI of bytes.
    fn sri(bytes: &[u8]) -> String {
        let digest = Sha384::digest(bytes);
        format!(
            "sha384-{}",
            base64::engine::general_purpose::STANDARD.encode(digest)
        )
    }

    /// True when `path` has the extension `ext` (any case).
    fn has_ext(path: &str, ext: &str) -> bool {
        std::path::Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    }

    /// The bundled bytes of `path` as text.
    fn text(path: &str) -> &'static str {
        let asset = PIXI_ASSETS.get(path).expect("file is bundled");
        std::str::from_utf8(asset.bytes()).expect("bundled text is UTF-8")
    }

    #[test]
    fn bundle_holds_exactly_the_served_files() {
        let mut files: Vec<&str> = PIXI_ASSETS
            .iter()
            .map(autumn_web::assets::PluginAsset::logical_path)
            .collect();
        files.sort_unstable();
        let mut expected = vec![PIXI_JS, UNSAFE_EVAL_JS, PARSE_JS, INIT_JS, PIXI_CSS];
        expected.sort_unstable();
        // `manifest.json` and `PIXI-LICENSE` are not served.
        assert_eq!(files, expected);
        assert_eq!(PIXI_ASSETS.namespace(), ASSETS_NAMESPACE);
        assert_eq!(PIXI_ASSETS.mount_path(), "/static/_plugins/pixi");
    }

    #[test]
    fn bundle_integrity_matches_embedded_bytes() {
        assert!(PIXI_ASSETS.iter().count() > 0, "bundle is not empty");
        for asset in PIXI_ASSETS.iter() {
            assert_eq!(
                asset.integrity(),
                sri(asset.bytes()),
                "{}",
                asset.logical_path()
            );
        }
    }

    #[test]
    fn vendored_files_are_unchanged_upstream_bytes() {
        let paths: Vec<&str> = VENDORED.iter().map(|f| f.path).collect();
        assert_eq!(paths, [PIXI_JS, UNSAFE_EVAL_JS]);
        for file in VENDORED {
            let asset = PIXI_ASSETS.get(file.path).expect("vendored file is bundled");
            assert_eq!(sri(asset.bytes()), file.upstream_integrity, "{}", file.path);
            assert!(
                file.source.starts_with(&format!(
                    "https://cdn.jsdelivr.net/npm/pixi.js@{PIXI_VERSION}/dist/"
                )),
                "{}",
                file.source
            );
            assert!(file.source.ends_with(file.path), "{}", file.source);
        }
    }

    #[test]
    fn vendored_files_are_the_pinned_version() {
        // Each upstream file starts with the PixiJS banner of its version.
        for file in VENDORED {
            let banner = format!("PixiJS - v{PIXI_VERSION}");
            assert!(text(file.path)[..200].contains(&banner), "{}", file.path);
        }
    }

    #[test]
    fn pixi_build_is_the_iife_that_sets_the_global() {
        assert!(text(PIXI_JS).contains("var PIXI="), "IIFE build");
        // The unsafe-eval package patches the global and installs itself.
        let patch = text(UNSAFE_EVAL_JS);
        assert!(patch.contains("PIXI.AbstractRenderer.prototype"), "patch");
        assert!(patch.contains("_unsafeEvalCheck(){}"), "no eval check");
    }

    #[test]
    fn plugin_files_use_no_eval_and_no_bare_imports() {
        for path in [PARSE_JS, INIT_JS] {
            let source = text(path);
            for banned in ["eval(", "new Function", "importmap", "from \"pixi"] {
                assert!(!source.contains(banned), "{path}: {banned}");
            }
        }
        // init.js imports only parse.js, at a relative URL.
        assert!(text(INIT_JS).contains("from \"./parse.js\";"));
    }

    #[test]
    fn urls_are_fingerprinted_under_the_plugin_mount() {
        assert!(PIXI_ASSETS.iter().count() > 0, "bundle is not empty");
        for asset in PIXI_ASSETS.iter() {
            let path = asset.logical_path();
            assert_eq!(asset.plain_url(), format!("/static/_plugins/pixi/{path}"));
            let (stem, ext) = path.rsplit_once('.').expect("extension");
            let hash = asset
                .url()
                .strip_prefix(&format!("/static/_plugins/pixi/{stem}."))
                .and_then(|rest| rest.strip_suffix(&format!(".{ext}")))
                .expect("fingerprinted form");
            assert_eq!(hash.len(), 8);
            assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn content_types_match_the_files() {
        assert!(PIXI_ASSETS.iter().count() > 0, "bundle is not empty");
        for asset in PIXI_ASSETS.iter() {
            let expected = if has_ext(asset.logical_path(), "css") {
                "text/css; charset=utf-8"
            } else {
                "text/javascript; charset=utf-8"
            };
            assert_eq!(asset.content_type(), expected, "{}", asset.logical_path());
        }
    }

    #[test]
    fn manifest_agrees_with_constants() {
        let manifest = include_str!("../assets/manifest.json");
        assert!(manifest.contains(&format!("\"version\": \"{PIXI_VERSION}\"")));
        assert!(!VENDORED.is_empty());
        for file in VENDORED {
            assert!(manifest.contains(file.source), "{}", file.source);
            assert!(
                manifest.contains(file.upstream_integrity),
                "{}",
                file.upstream_integrity
            );
        }
    }

    #[test]
    fn vendor_script_agrees_with_constants() {
        let script = include_str!("../scripts/vendor.sh");
        assert!(script.contains(&format!("VERSION=\"{PIXI_VERSION}\"")));
        assert!(!VENDORED.is_empty());
        for file in VENDORED {
            let hash = file.upstream_integrity.trim_start_matches("sha384-");
            assert!(script.contains(hash), "{} pin in vendor.sh", file.path);
        }
    }

    #[test]
    fn license_file_is_vendored_but_not_served() {
        let license = include_str!("../assets/PIXI-LICENSE");
        assert!(license.starts_with("The MIT License"));
        assert!(PIXI_ASSETS.get("PIXI-LICENSE").is_none());
    }
}
