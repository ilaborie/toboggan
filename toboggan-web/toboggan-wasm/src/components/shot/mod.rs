//! One slide, held still, for a screenshot.
//!
//! The thumbnails behind `/slides` used to be a second rendering of the deck —
//! the same markdown put through a Typst document instead of a browser. It could
//! only ever approximate: `<style>` blocks, raw HTML and terminals have no Typst
//! equivalent, so a deck that leans on any of them had thumbnails that did not
//! match the projector. This page *is* the deck, in a headless browser, so the
//! thumbnail and the slide are the same rendering by construction.
//!
//! It is a mirror without a presenter: [`DeckPainter`] does the painting, the
//! frame comes from the REST API rather than `postMessage`, and — as with a
//! mirror — no socket is opened and no client registers, so shooting a deck does
//! not appear in `/api/clients` or move the room.
//!
//! Two things the driver relies on, which is why they are not incidental:
//!
//! * every reveal is shown at once ([`MirrorFrame::step`] is `None`), because a
//!   thumbnail of a half-built slide is a thumbnail of nothing;
//! * the page announces itself finished by putting [`SHOT_ATTRIBUTE`] on
//!   `<html>`, so the screenshot is taken against settled fonts and decoded
//!   images rather than against a timeout. The failure value matters as much as
//!   the success one: without it a deck that cannot be fetched is captured as a
//!   blank rectangle and filed as a thumbnail.

use gloo::console::{error, info};
use gloo::utils::{document, window};
use toboggan_core::SlideId;
use wasm_bindgen::JsCast as _;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Element, HtmlElement, HtmlImageElement, NodeList};

use crate::components::WasmElement;
use crate::components::deck::DeckPainter;
use crate::services::TobogganApi;
use crate::services::mirror::MirrorFrame;

/// The attribute the driver polls, on `<html>`: `ready` or `error`.
pub(crate) const SHOT_ATTRIBUTE: &str = "data-toboggan-shot";

/// How many animation frames to wait for images before shooting anyway.
///
/// A budget, not a deadline to meet: a slide referencing an image the deck does
/// not ship would otherwise never report ready, and one missing picture is a
/// poor reason to lose the whole overview. Exceeding it is logged and still
/// counts as `ready`, because what the browser has by then is what a person
/// opening that slide would see too.
///
/// Counted in frames rather than milliseconds because the loop already waits on
/// `requestAnimationFrame` — about five seconds at sixty of them a second, and
/// self-adjusting on a machine that cannot manage sixty.
const SETTLE_BUDGET_FRAMES: usize = 300;

/// A deck page showing exactly one slide, fully revealed, and nothing else.
pub(crate) struct ShotApp {
    index: usize,
}

impl ShotApp {
    pub(crate) const fn new(index: usize) -> Self {
        Self { index }
    }
}

impl WasmElement for ShotApp {
    fn render(&mut self, host: &HtmlElement) {
        hold_still();

        let mut painter = DeckPainter::mount(host);
        let index = self.index;

        spawn_local(async move {
            match load_frame(index).await {
                Some(frame) => {
                    painter.show(frame);
                    settle().await;
                    announce("ready");
                }
                None => announce("error"),
            }
        });
    }
}

/// Fetches everything the frame needs, or `None` if any of it is unavailable.
async fn load_frame(index: usize) -> Option<MirrorFrame> {
    // Same-origin: the shot page is served by the very server being shot, and an
    // empty base leaves the API paths rooted at `/`.
    let api = TobogganApi::new("");

    let talk = match api.get_talk().await {
        Ok(talk) => talk,
        Err(err) => {
            error!("Shot could not fetch the talk:", err.to_string());
            return None;
        }
    };
    let slide = match api.get_slide(SlideId::new(index)).await {
        Ok(slide) => slide,
        Err(err) => {
            error!("Shot could not fetch slide:", index, err.to_string());
            return None;
        }
    };

    Some(MirrorFrame {
        head: talk.head,
        footer: talk.footer,
        lang: talk.lang,
        slide: Some(slide),
        // Every reveal at once — the same request the presenter's next-slide
        // pane makes, and for the same reason.
        step: None,
        // The class a slide on the projector renders under. `state.css` hangs
        // the entrance animation off it, which `hold_still` has already
        // neutralised.
        state_class: "running".to_owned(),
        slide_number: SlideId::new(index).display_number(),
        total_slides: talk.titles.len(),
    })
}

/// Takes the motion out of the document.
///
/// Nothing here is decoration: `state.css` gives `.running .toboggan-slide` a
/// 0.7s entrance that translates, scales *and blurs* the slide, so a screenshot
/// taken during it is a smeared slide sliding in from the right. The step
/// transition inside the slide's shadow root is the same hazard one boundary
/// down, and a document rule cannot reach it — a custom property can, because
/// custom properties inherit through a shadow boundary while selectors do not.
///
/// Applied before the deck is mounted, so there is no first frame to animate.
fn hold_still() {
    let Ok(Some(head)) = document().query_selector("head") else {
        error!("Shot page has no <head>; the deck will animate under the camera");
        return;
    };
    let style = document().create_element("style");
    let Ok(style) = style else {
        return;
    };
    style.set_text_content(Some(
        ":root {
  --transition-fast: 0s;
  --animation-fast: 0s;
  --step-transition-duration: 0s;
}
*, *::before, *::after {
  animation-duration: 0s !important;
  animation-delay: 0s !important;
  animation-iteration-count: 1 !important;
  transition-duration: 0s !important;
  transition-delay: 0s !important;
  caret-color: transparent !important;
}",
    ));
    let _ = head.append_child(&style);
}

/// Waits for the things that make a screenshot honest: web fonts, and images
/// that have actually decoded.
///
/// Fonts first and separately, because a slide laid out in the fallback face
/// breaks its lines somewhere else — the same reason a mirror carries the deck's
/// language tag.
async fn settle() {
    if let Ok(ready) = document().fonts().ready()
        && let Err(err) = JsFuture::from(ready).await
    {
        error!("Shot fonts never settled:", err);
    }

    let mut frames = 0;
    while pending_images() > 0 {
        if frames >= SETTLE_BUDGET_FRAMES {
            info!(
                "Shot gave up waiting on",
                pending_images(),
                "image(s); capturing what the browser has"
            );
            break;
        }
        next_frame().await;
        frames += 1;
    }

    // Two more frames after the last change: one for the style and layout the
    // painting queued, one to be sure it has been through paint.
    next_frame().await;
    next_frame().await;
}

/// How many `<img>` elements are still loading, shadow roots included.
///
/// A plain `document.querySelectorAll("img")` misses every one that matters:
/// the slide is rendered into a shadow root, and that is where a deck's pictures
/// live. `querySelectorAll` does not cross that boundary, so the hosts are
/// enumerated and each open root asked separately.
fn pending_images() -> usize {
    fn count_loading(images: &NodeList) -> usize {
        (0..images.length())
            .filter_map(|index| images.item(index))
            .filter_map(|node| node.dyn_into::<HtmlImageElement>().ok())
            .filter(|image| !image.complete())
            .count()
    }

    let document = document();
    let mut pending = document
        .query_selector_all("img")
        .map(|images| count_loading(&images))
        .unwrap_or_default();

    let Ok(hosts) = document.query_selector_all("*") else {
        return pending;
    };
    for index in 0..hosts.length() {
        let Some(shadow) = hosts
            .item(index)
            .and_then(|node| node.dyn_into::<Element>().ok())
            .and_then(|element| element.shadow_root())
        else {
            continue;
        };
        if let Ok(images) = shadow.query_selector_all("img") {
            pending += count_loading(&images);
        }
    }
    pending
}

/// Resolves on the next animation frame.
async fn next_frame() {
    let (sender, receiver) = futures::channel::oneshot::channel::<()>();
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        let _ = sender.send(());
    });
    if window()
        .request_animation_frame(callback.unchecked_ref())
        .is_err()
    {
        return;
    }
    let _ = receiver.await;
}

/// Publishes the outcome where the screenshot driver can see it.
fn announce(outcome: &str) {
    let Some(root) = document().document_element() else {
        return;
    };
    if let Err(err) = root.set_attribute(SHOT_ATTRIBUTE, outcome) {
        error!("Shot could not announce itself:", err);
    }
}
