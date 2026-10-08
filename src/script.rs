//! Tag helpers: [`pixi_script()`] and [`pixi_stylesheet()`].
//!
//! [`pixi_script()`] emits, in this order:
//!
//! 1. `<script defer>` for `pixi.min.js` (sets the global `PIXI`).
//! 2. `<script defer>` for `unsafe-eval.min.js` (patches `PIXI`, so that
//!    PixiJS needs no `eval`).
//! 3. `<link rel="modulepreload">` for `parse.js`. `init.js` imports it.
//! 4. `<script type="module">` for `init.js`.
//!
//! Each tag has SRI. The scripts use hashed URLs, except the preload:
//! module imports resolve to plain URLs. The browser runs `defer` and module
//! scripts in document order after parsing. Thus `PIXI` exists before
//! `init.js` runs. There is no import map and no inline script, so the
//! default Autumn CSP (`script-src 'self'`) allows all tags.

use autumn_web::{Markup, html};

use crate::assets::{INIT_JS, PARSE_JS, PIXI_ASSETS, PIXI_CSS, PIXI_JS, UNSAFE_EVAL_JS};

/// Renders the tags that load PixiJS and the plugin runtime.
///
/// Put it in the page `<head>`. Your own `defer` and module scripts can use
/// the global `PIXI` when they come after these tags.
///
/// ```rust
/// use autumn_plugin_pixijs::pixi_script;
/// use autumn_web::html;
///
/// let head = html! { head { (pixi_script()) } }.into_string();
/// assert!(head.contains(r#"type="module""#));
/// assert!(head.contains("defer"));
/// ```
#[must_use]
pub fn pixi_script() -> Markup {
    html! {
        @for path in [PIXI_JS, UNSAFE_EVAL_JS] {
            @if let Some(asset) = PIXI_ASSETS.get(path) {
                script src=(asset.url()) integrity=(asset.integrity())
                    crossorigin="anonymous" defer {}
            }
        }
        @if let Some(parse) = PIXI_ASSETS.get(PARSE_JS) {
            link rel="modulepreload" href=(parse.plain_url())
                integrity=(parse.integrity()) crossorigin="anonymous";
        }
        @if let Some(init) = PIXI_ASSETS.get(INIT_JS) {
            script type="module" src=(init.url()) integrity=(init.integrity())
                crossorigin="anonymous" {}
        }
    }
}

/// Renders the `<link>` tag for the plugin stylesheet.
///
/// The stylesheet gives stages a default size (`aspect-ratio: 16 / 9`),
/// hides object declarations, and shows fallback content when needed. Put
/// it in the page `<head>`.
///
/// ```rust
/// use autumn_plugin_pixijs::pixi_stylesheet;
///
/// let html = pixi_stylesheet().into_string();
/// assert!(html.contains(r#"href="/static/_plugins/pixi/pixi."#), "{html}");
/// ```
#[must_use]
pub fn pixi_stylesheet() -> Markup {
    PIXI_ASSETS.stylesheet_tag(PIXI_CSS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(path: &str) -> &'static autumn_web::assets::PluginAsset {
        PIXI_ASSETS.get(path).expect("file is bundled")
    }

    fn classic(path: &str) -> String {
        let a = asset(path);
        format!(
            r#"<script src="{}" integrity="{}" crossorigin="anonymous" defer></script>"#,
            a.url(),
            a.integrity()
        )
    }

    #[test]
    fn pixi_and_unsafe_eval_are_deferred_classic_scripts_with_sri() {
        let html = pixi_script().into_string();
        for path in [PIXI_JS, UNSAFE_EVAL_JS] {
            assert!(html.contains(&classic(path)), "{path}: {html}");
        }
    }

    #[test]
    fn parse_js_is_preloaded_with_sri_at_its_plain_url() {
        let html = pixi_script().into_string();
        let a = asset(PARSE_JS);
        let tag = format!(
            r#"<link rel="modulepreload" href="{}" integrity="{}" crossorigin="anonymous">"#,
            a.plain_url(),
            a.integrity()
        );
        assert!(html.contains(&tag), "{html}");
    }

    #[test]
    fn entry_is_a_module_script_with_sri_and_fingerprinted_url() {
        let html = pixi_script().into_string();
        let init = asset(INIT_JS);
        let tag = format!(
            r#"<script type="module" src="{}" integrity="{}" crossorigin="anonymous"></script>"#,
            init.url(),
            init.integrity()
        );
        assert!(html.contains(&tag), "{html}");
        assert_eq!(html.matches("<script").count(), 3, "three scripts: {html}");
        assert!(!html.contains("importmap"), "no inline import map: {html}");
        assert!(!html.contains("></script><script>"), "no inline script");
    }

    #[test]
    fn tags_come_in_load_order() {
        let html = pixi_script().into_string();
        let at = |needle: &str| html.find(needle).expect("tag present");
        let pixi = at(&classic(PIXI_JS));
        let patch = at(&classic(UNSAFE_EVAL_JS));
        let preload = at(asset(PARSE_JS).plain_url());
        let init = at(asset(INIT_JS).url());
        assert!(pixi < patch && patch < preload && preload < init, "{html}");
    }

    #[test]
    fn stylesheet_link_carries_sri_and_fingerprinted_url() {
        let html = pixi_stylesheet().into_string();
        let css = asset(PIXI_CSS);
        assert!(html.contains(r#"rel="stylesheet""#), "{html}");
        assert!(html.contains(&format!(r#"href="{}""#, css.url())), "{html}");
        assert!(
            html.contains(&format!(r#"integrity="{}""#, css.integrity())),
            "{html}"
        );
    }
}
