//! PixiJS demo: a small Autumn app that shows `autumn-plugin-pixijs`.
//!
//! ```sh
//! cargo run --example pixi_demo
//! ```
//!
//! Open <http://127.0.0.1:3000>. The page shows:
//!
//! - a hero stage: a scrolling star field, a spinning star, a sprite-sheet
//!   animation, and text,
//! - a coin game: each tap sends an htmx request, and the server sends the
//!   next stage,
//! - an htmx gallery: each click gets a new stage from the server, and the
//!   runtime frees the old stage,
//! - a stage written as raw `data-pixi-*` attributes,
//! - custom JavaScript that uses the `pixi:ready` event.
//!
//! No inline script and no inline style: all JS and CSS are external files,
//! so the default Autumn CSP and nonce mode both work.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use autumn_plugin_pixijs::{
    Align, AnimatedSprite, Color, PixiPlugin, Shape, Sprite, Stage, Text, TilingSprite, Vec2,
    pixi_script, pixi_stylesheet,
};
use autumn_web::assets::asset_url;
use autumn_web::security::CsrfToken;
use autumn_web::{Markup, html};

/// The crate `static/` dir: demo CSS, demo JS, and images.
static STATIC: autumn_web::include_dir::Dir = autumn_web::embed_static!();

/// Gallery position for the `/shape` partial.
static SHAPE: AtomicUsize = AtomicUsize::new(0);

/// Coin positions in the game stage.
const COINS: [[f32; 2]; 5] = [
    [120.0, 110.0],
    [300.0, 300.0],
    [420.0, 140.0],
    [560.0, 320.0],
    [690.0, 120.0],
];

/// Which coins the player collected. Demo only: one game for all users.
static COLLECTED: Mutex<[bool; COINS.len()]> = Mutex::new([false; COINS.len()]);

#[autumn_web::main]
async fn main() {
    autumn_web::app()
        .plugin(PixiPlugin::new())
        .embedded_static(&STATIC)
        .routes(autumn_web::routes![index, shape, collect, reset])
        .run()
        .await;
}

/// The page shell. With CSRF on (the `prod` profile), the token tag and the
/// Autumn htmx CSRF helper let `tap_post` requests pass.
fn layout(csrf: Option<&CsrfToken>, content: &Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                // htmx adds an inline <style> for indicators. CSP nonce mode
                // blocks it, so turn it off.
                meta name="htmx-config" content=r#"{"includeIndicatorStyles":false}"#;
                title { "PixiJS demo" }
                @if let Some(token) = csrf {
                    meta name="csrf-token" content=(token.token());
                }
                script src=(asset_url("js/autumn-htmx-csrf.js")) defer {}
                link rel="stylesheet" href=(asset_url("css/demo.css"));
                (pixi_stylesheet())
                (pixi_script())
                script src=(asset_url("js/htmx.min.js")) defer {}
                script type="module" src=(asset_url("js/demo.js")) {}
            }
            body { main { (content) } }
        }
    }
}

/// The current coin collection.
fn collected() -> [bool; COINS.len()] {
    *COLLECTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The coin game stage. The server owns the game state.
fn game(collected: [bool; COINS.len()]) -> Markup {
    let score = collected.iter().filter(|c| **c).count();
    let mut stage = Stage::new()
        .id("game")
        .label("Coin game: tap the coins")
        .background(Color::hex(0x0014_532d))
        .add(
            Text::new(format!("Score: {score} / {}", COINS.len()))
                .id("score")
                .font_size(32.0)
                .fill(Color::WHITE)
                .bold()
                .anchor([0.0, 0.0])
                .position([24.0, 20.0]),
        );
    for (i, [x, y]) in COINS.into_iter().enumerate() {
        if !collected[i] {
            stage = stage.add(
                Sprite::new("/static/img/coin.png")
                    .id(format!("coin-{i}"))
                    .label(format!("Coin {}", i + 1))
                    .position([x, y])
                    .scale(Vec2::splat(1.4))
                    .spin(if i % 2 == 0 { 30.0 } else { -30.0 })
                    .tap_post(format!("/collect/{i}"))
                    .tap_target("#game")
                    .tap_swap("outerHTML"),
            );
        }
    }
    if score == COINS.len() {
        stage = stage.add(
            Text::new("You found them all!")
                .font_size(48.0)
                .fill(Color::hex(0x00fd_e047))
                .bold(),
        );
    }
    html! { (stage) }
}

#[autumn_web::get("/")]
async fn index(csrf: Option<CsrfToken>) -> Markup {
    layout(
        csrf.as_ref(),
        &html! {
            section class="hero" {
                div {
                    p class="kicker" { "autumn-plugin-pixijs" }
                    h1 { "2D canvas in server-rendered HTML." }
                    p { "Each stage on this page is Maud markup. Tap a coin." }
                }
                (Stage::new()
                    .id("hero")
                    .class("hero-stage")
                    .label("A star field with a spinning orange star")
                    .size(600.0, 400.0)
                    .add(TilingSprite::new("/static/img/stars.png", 600.0, 400.0).scroll(-20.0, 8.0))
                    .add(Shape::star(5, 110.0, 50.0).fill(Color::hex(0x00ff_7a18)).stroke(Color::WHITE, 4.0).spin(20.0))
                    .add(AnimatedSprite::new("/static/img/pulse.json").fps(8.0).position([480.0, 300.0]))
                    .add(Text::new("PixiJS 8").font_size(40.0).fill(Color::WHITE).bold().position([300.0, 360.0]))
                    .fallback(html! { p class="fallback" { "This stage needs JavaScript." } }))
            }
            section {
                h2 { "Coin game (htmx)" }
                p { "A tap sends " code { "POST /collect/{n}" } ". The server sends the next stage." }
                (game(collected()))
                button hx-post="/reset" hx-target="#game" hx-swap="outerHTML" { "Reset" }
            }
            section {
                h2 { "Gallery (htmx)" }
                button hx-get="/shape" hx-target="#gallery" hx-swap="innerHTML" { "Next shape" }
                div id="gallery" { (gallery_stage(0)) }
            }
            section class="pair" {
                div {
                    h2 { "Raw attributes" }
                    (maud::PreEscaped(r##"<div data-pixi="stage" data-pixi-size="400,240" data-pixi-background="#1e293b" role="img" aria-label="Three shapes">
  <div hidden data-pixi-shape="circle" data-pixi-args="50" data-pixi-fill="#38bdf8" data-pixi-position="100,120"></div>
  <div hidden data-pixi-shape="rounded-rect" data-pixi-args="100,100,16" data-pixi-fill="#a78bfa" data-pixi-position="200,120" data-pixi-spin="45"></div>
  <div hidden data-pixi-shape="polygon" data-pixi-args="0,-50,50,40,-50,40" data-pixi-fill="#f472b6" data-pixi-position="300,120"></div>
</div>"##))
                }
                div {
                    h2 { "Custom JavaScript" }
                    p { "static/js/demo.js adds a bouncing ball in " code { "pixi:ready" } "." }
                    (Stage::new()
                        .id("custom")
                        .size(400.0, 240.0)
                        .background(Color::hex(0x000f_172a))
                        .label("A ball that bounces")
                        .add(Text::new("Custom code").fill(Color::WHITE).align(Align::Center).position([200.0, 30.0])))
                }
            }
        },
    )
}

/// One stage of the gallery.
fn gallery_stage(n: usize) -> Markup {
    let colors = [0x00f4_3f5e, 0x0022_c55e, 0x003b_82f6, 0x00ea_b308];
    let shapes = [
        Shape::star(7, 90.0, 40.0),
        Shape::rounded_rect(180.0, 120.0, 24.0),
        Shape::ellipse(110.0, 60.0),
        Shape::polygon([[0.0, -90.0], [90.0, 70.0], [-90.0, 70.0]]),
    ];
    let shape = shapes[n % shapes.len()].clone();
    html! {
        (Stage::new()
            .size(400.0, 240.0)
            .label(format!("Gallery shape {}", n + 1))
            .add(shape.fill(Color::hex(colors[n % colors.len()])).spin(40.0)))
    }
}

#[autumn_web::get("/shape")]
async fn shape() -> Markup {
    let n = SHAPE.fetch_add(1, Ordering::Relaxed) + 1;
    gallery_stage(n)
}

#[autumn_web::post("/collect/{n}")]
async fn collect(n: autumn_web::extract::Path<usize>) -> Markup {
    let mut state = COLLECTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(coin) = state.get_mut(n.0) {
        *coin = true;
    }
    game(*state)
}

#[autumn_web::post("/reset")]
async fn reset() -> Markup {
    let mut state = COLLECTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *state = [false; COINS.len()];
    game(*state)
}
