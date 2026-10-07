//! Test fixture app for the browser E2E tests in `tests/e2e/`.
//!
//! Each route is one scenario. Run it with:
//!
//! ```sh
//! AUTUMN_SERVER__PORT=3111 cargo run --example e2e_fixture
//! ```
//!
//! Not a demo. See `examples/pixi_demo.rs` for the demo.

use std::sync::atomic::{AtomicUsize, Ordering};

use autumn_plugin_pixijs::{
    Align, AnimatedSprite, Color, PIXI_ASSETS, PixiPlugin, Renderer, Shape, Sprite, Stage, Text,
    TilingSprite, pixi_script, pixi_stylesheet,
};
use autumn_web::assets::asset_url;
use autumn_web::{Markup, html};

/// The crate `static/` dir: fixture images and scripts.
static STATIC: autumn_web::include_dir::Dir = autumn_web::embed_static!();

/// Tap count for `POST /tap`.
static TAPS: AtomicUsize = AtomicUsize::new(0);

#[autumn_web::main]
async fn main() {
    autumn_web::app()
        .plugin(PixiPlugin::new())
        .embedded_static(&STATIC)
        .routes(autumn_web::routes![
            basic,
            kinds,
            handwritten,
            size,
            motion,
            motion_animate,
            still,
            sprite_missing,
            sprite_missing_no_fallback,
            sprite_broken,
            sheet_bad_animation,
            swap,
            swap_next,
            offscreen,
            tap_page,
            tap,
            canvas,
            webgl_only,
            text,
            params,
            late_script,
            wrong_type,
        ])
        .run()
        .await;
}

/// The page shell.
fn page(content: &Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                title { "pixi e2e" }
                // htmx adds an inline <style> for indicators. CSP nonce mode
                // blocks it, so turn it off.
                meta name="htmx-config" content=r#"{"includeIndicatorStyles":false}"#;
                link rel="stylesheet" href=(asset_url("css/fixture.css"));
                (pixi_stylesheet())
                (pixi_script())
                script src=(asset_url("js/htmx.min.js")) defer {}
                script type="module" src=(asset_url("js/fixture-listener.js")) {}
            }
            body { (content) }
        }
    }
}

/// A red rectangle that fills the stage center. Pixel tests read it.
fn red_rect() -> Shape {
    Shape::rect(400.0, 200.0).fill(Color::hex(0x00ff_0000))
}

/// The fallback text.
fn fallback() -> Markup {
    html! { p class="fallback" { "No canvas here." } }
}

#[autumn_web::get("/basic")]
async fn basic() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .label("A red rectangle")
            .background(Color::hex(0x0000_00ff))
            .add(red_rect())
            .fallback(fallback()))
    })
}

#[autumn_web::get("/kinds")]
async fn kinds() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .add(Sprite::new("/static/img/red.png").id("sprite").position([100.0, 100.0]))
            .add(Sprite::new("/static/img/dot.svg").id("svg").position([250.0, 100.0]).size(32.0, 32.0))
            .add(TilingSprite::new("/static/img/checker.png", 200.0, 100.0).id("tiling").position([450.0, 100.0]))
            .add(AnimatedSprite::new("/static/img/blink.json").id("sheet").position([700.0, 100.0]))
            .add(Text::new("Hello").id("text").position([100.0, 300.0]).align(Align::Right).wrap(200.0))
            .add(Shape::rect(40.0, 40.0).id("rect").position([200.0, 300.0]))
            .add(Shape::rounded_rect(40.0, 40.0, 8.0).id("rounded").position([280.0, 300.0]))
            .add(Shape::circle(20.0).id("circle").position([360.0, 300.0]))
            .add(Shape::ellipse(30.0, 15.0).id("ellipse").position([440.0, 300.0]))
            .add(Shape::star(5, 25.0, 10.0).id("star").position([520.0, 300.0]))
            .add(Shape::polygon([[0.0, -20.0], [20.0, 20.0], [-20.0, 20.0]]).id("polygon").position([600.0, 300.0]))
            .add(Shape::circle(20.0).no_fill().stroke(Color::BLACK, 4.0).id("ring").position([680.0, 300.0])))
    })
}

#[autumn_web::get("/handwritten")]
async fn handwritten() -> Markup {
    page(&maud::PreEscaped(
        r##"<div id="stage" data-pixi="stage" data-pixi-background="#000000" data-pixi-renderer="gpu">
  <div hidden data-pixi-shape="teapot"></div>
  <div hidden data-pixi-sprite="javascript:alert(1)"></div>
  <div hidden data-pixi-shape="rect" data-pixi-args="400,200,x" data-pixi-fill="#00ff00" data-pixi-alpha="laser"></div>
</div>"##
            .to_owned(),
    ))
}

#[autumn_web::get("/size")]
async fn size() -> Markup {
    page(&html! {
        div class="narrow" {
            (Stage::new().id("square").size(300.0, 300.0).add(Shape::circle(100.0)))
        }
        div class="narrow" {
            (Stage::new().id("custom").size(800.0, 400.0).add(Shape::circle(100.0)))
        }
    })
}

/// A stage with every kind of automatic motion.
fn motion_stage() -> Stage {
    Stage::new()
        .id("stage")
        .add(red_rect().id("spinner").spin(180.0))
        .add(
            TilingSprite::new("/static/img/checker.png", 100.0, 100.0)
                .id("tiles")
                .scroll(60.0, -30.0),
        )
        .add(
            AnimatedSprite::new("/static/img/blink.json")
                .id("sheet")
                .fps(30.0),
        )
}

#[autumn_web::get("/motion")]
async fn motion() -> Markup {
    page(&html! { (motion_stage()) })
}

#[autumn_web::get("/motion-animate")]
async fn motion_animate() -> Markup {
    page(&html! { (motion_stage().animate_reduced_motion()) })
}

#[autumn_web::get("/still")]
async fn still() -> Markup {
    page(&html! { (Stage::new().id("stage").add(red_rect())) })
}

#[autumn_web::get("/sprite-missing")]
async fn sprite_missing() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .add(red_rect())
            .add(Sprite::new("/static/img/missing.png"))
            .fallback(fallback()))
    })
}

#[autumn_web::get("/sprite-missing-no-fallback")]
async fn sprite_missing_no_fallback() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .background(Color::hex(0x0000_00ff))
            .add(red_rect())
            .add(Sprite::new("/static/img/missing.png")))
    })
}

#[autumn_web::get("/sprite-broken")]
async fn sprite_broken() -> Markup {
    page(&html! {
        (Stage::new().id("stage").add(Sprite::new("/static/img/broken.png")).fallback(fallback()))
    })
}

#[autumn_web::get("/sheet-bad-animation")]
async fn sheet_bad_animation() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .add(AnimatedSprite::new("/static/img/blink.json").id("sheet").animation("nope")))
    })
}

#[autumn_web::get("/swap")]
async fn swap() -> Markup {
    page(&html! {
        button id="next" hx-get="/swap/next" hx-target="#slot" hx-swap="innerHTML" { "Next" }
        div id="slot" {
            (Stage::new().id("first").add(red_rect().spin(90.0)))
        }
    })
}

#[autumn_web::get("/swap/next")]
async fn swap_next() -> Markup {
    html! {
        (Stage::new().id("second").add(Shape::circle(80.0).fill(Color::hex(0x0000_ff00))))
    }
}

#[autumn_web::get("/offscreen")]
async fn offscreen() -> Markup {
    page(&html! {
        div id="spacer" {}
        (Stage::new().id("stage").add(red_rect().spin(90.0)))
    })
}

#[autumn_web::get("/tap")]
async fn tap_page() -> Markup {
    TAPS.store(0, Ordering::SeqCst);
    page(&html! {
        (Stage::new()
            .id("stage")
            .add(
                red_rect()
                    .id("button")
                    .label("Press me")
                    .tap_post("/tap")
                    .tap_target("#log")
                    .tap_swap("innerHTML"),
            )
            .add(Shape::circle(30.0).id("plain").position([60.0, 60.0]).tappable()))
        div id="log" { "none" }
    })
}

#[autumn_web::post("/tap")]
async fn tap() -> Markup {
    let n = TAPS.fetch_add(1, Ordering::SeqCst) + 1;
    html! { "tapped " (n) }
}

#[autumn_web::get("/canvas")]
async fn canvas() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .renderer(Renderer::Canvas)
            .background(Color::hex(0x0000_00ff))
            .add(red_rect())
            .fallback(fallback()))
    })
}

#[autumn_web::get("/webgl-only")]
async fn webgl_only() -> Markup {
    page(&html! {
        (Stage::new().id("stage").renderer(Renderer::WebGl).add(red_rect()).fallback(fallback()))
    })
}

#[autumn_web::get("/text")]
async fn text() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .add(Text::new("Score: 1").id("score").font_size(40.0).fill(Color::WHITE).bold()))
    })
}

#[autumn_web::get("/params")]
async fn params() -> Markup {
    page(&html! {
        (Stage::new()
            .id("stage")
            .size(400.0, 200.0)
            .add(
                Shape::rect(100.0, 50.0)
                    .id("rect")
                    .position([50.0, 60.0])
                    .rotation(90.0)
                    .scale([2.0, 3.0])
                    .anchor([0.0, 1.0])
                    .alpha(0.5)
                    .tint(Color::hex(0x00ff_0000)),
            )
            .add(Sprite::new("/static/img/red.png").id("sprite").size(20.0, 10.0).anchor([1.0, 0.0]))
            .add(TilingSprite::new("/static/img/checker.png", -1.0, 0.0).id("tiling"))
            .add(
                Text::new("x")
                    .id("text")
                    .font_size(30.0)
                    .font_family("monospace")
                    .fill(Color::hex(0x0012_3456))
                    .bold()
                    .align(Align::Center)
                    .wrap(120.0),
            )
            .add(Shape::star(6, 40.0, 20.0).id("star").fill(Color::hex(0x00ab_cdef)).stroke(Color::BLACK, 5.0)))
    })
}

/// PixiJS loads, but `init.js` does not. The test adds `init.js` later.
#[autumn_web::get("/late-script")]
async fn late_script() -> Markup {
    html! {
        (maud::DOCTYPE)
        html {
            head {
                meta charset="utf-8";
                (pixi_stylesheet())
                script src=(PIXI_ASSETS.url("pixi.min.js")) defer {}
                script src=(PIXI_ASSETS.url("unsafe-eval.min.js")) defer {}
            }
            body data-init=(PIXI_ASSETS.url("init.js")) {
                (Stage::new().id("stage").add(red_rect()))
            }
        }
    }
}

/// Assets of the wrong type. The path selects the case.
#[autumn_web::get("/wrong-type/{kind}")]
async fn wrong_type(kind: autumn_web::extract::Path<String>) -> Markup {
    let object: autumn_plugin_pixijs::StageObject = match kind.0.as_str() {
        "sprite" => Sprite::new("/static/img/blink.json").into(),
        "sheet" => AnimatedSprite::new("/static/img/red.png").into(),
        _ => AnimatedSprite::new("/static/img/frames.json").into(),
    };
    page(&html! { (Stage::new().id("stage").add(object).fallback(fallback())) })
}
