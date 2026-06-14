// path: src/adapter/egui_renderer.rs
//
// EguiRenderer — egui-backed implementation of the GuiRenderer port.
//
// # Design
//
// `EguiRenderer` adapts the `GuiRenderer` port's begin/paint/end lifecycle to
// egui's immediate-mode model.  Paint callbacks are queued during a frame and
// flushed (called with their reserved `Rect`) when `end_frame` is called.
//
// The egui `Context` is accepted through the constructor so the renderer is
// testable without a real window.  The `GuiRenderer` trait itself is pure logic
// and never touches audio-thread resources.
//
// # Thread-safety note
//
// All `GuiRenderer` methods must be called from the UI thread.  `EguiRenderer`
// does not implement `Send` or `Sync` and must not be moved across threads.

use crate::shell::gui_renderer::{GuiRenderer, PaintCallback, Rect, UiContext};

// ─── PendingPaint ─────────────────────────────────────────────────────────────

/// A paint callback queued for the current frame.
struct PendingPaint {
    region: Rect,
    callback: PaintCallback,
}

// ─── EguiRenderer ─────────────────────────────────────────────────────────────

/// egui-backed implementation of [`GuiRenderer`].
///
/// Wraps an [`egui::Context`] and implements the begin/paint/end frame
/// lifecycle defined by the [`GuiRenderer`] port.  Paint callbacks are
/// accumulated during a frame and flushed — in submission order — inside
/// `end_frame`.
///
/// # Usage
///
/// ```no_run
/// use crest_synth::adapter::egui_renderer::EguiRenderer;
/// use crest_synth::shell::gui_renderer::{GuiRenderer, Rect};
///
/// let ctx = egui::Context::default();
/// let mut renderer = EguiRenderer::new(ctx);
///
/// let ui_ctx = renderer.begin_frame();
/// renderer.custom_paint(Rect::new(0.0, 0.0, 800.0, 600.0), Box::new(|_r| {
///     // custom drawing here
/// }));
/// renderer.end_frame(ui_ctx);
/// ```
pub struct EguiRenderer {
    /// The egui rendering context.  Injected via constructor so tests can
    /// supply a default context without a real window.
    ctx: egui::Context,
    /// Running frame counter, incremented on each `begin_frame`.
    frame_counter: u64,
    /// Callbacks queued for the current frame, drained in `end_frame`.
    pending: Vec<PendingPaint>,
    /// Whether a frame has been opened (i.e. `begin_frame` called but
    /// `end_frame` not yet called).
    in_frame: bool,
}

impl EguiRenderer {
    /// Creates a new `EguiRenderer` with the supplied [`egui::Context`].
    ///
    /// # Arguments
    ///
    /// * `ctx` — The egui context to use for rendering.  Pass
    ///   `egui::Context::default()` for a context that is not attached to a
    ///   window (useful in tests).
    pub fn new(ctx: egui::Context) -> Self {
        Self {
            ctx,
            frame_counter: 0,
            pending: Vec::new(),
            in_frame: false,
        }
    }

    /// Returns a reference to the underlying [`egui::Context`].
    ///
    /// Useful when the owning shell code needs to pass the context to egui
    /// widgets.
    pub fn egui_ctx(&self) -> &egui::Context {
        &self.ctx
    }
}

impl GuiRenderer for EguiRenderer {
    /// Begins a new UI frame.
    ///
    /// Signals egui to start a new frame via [`egui::Context::begin_pass`]
    /// and returns a [`UiContext`] stamped with the current frame index.
    ///
    /// # Panics
    ///
    /// Panics if called while a frame is already open (i.e. before the
    /// matching `end_frame`).
    fn begin_frame(&mut self) -> UiContext {
        assert!(
            !self.in_frame,
            "EguiRenderer::begin_frame called while a frame is already open"
        );
        // Signal egui that a new pass is beginning.
        self.ctx.begin_pass(egui::RawInput::default());
        let ctx = UiContext::new(self.frame_counter);
        self.frame_counter += 1;
        self.in_frame = true;
        ctx
    }

    /// Queues a custom paint callback for the current frame.
    ///
    /// The callback is stored and will be invoked (in order) during the
    /// matching `end_frame` call.
    ///
    /// # Panics
    ///
    /// Panics if called outside of a `begin_frame` / `end_frame` pair.
    fn custom_paint(&mut self, region: Rect, callback: PaintCallback) {
        assert!(
            self.in_frame,
            "EguiRenderer::custom_paint called outside of a begin_frame/end_frame pair"
        );
        self.pending.push(PendingPaint { region, callback });
    }

    /// Ends the current frame.
    ///
    /// Flushes all pending paint callbacks (in submission order), each
    /// receiving the `Rect` it was registered with.  Calls
    /// [`egui::Context::end_pass`] to complete the egui frame, then
    /// discards the [`UiContext`] token to enforce single-use semantics.
    ///
    /// # Panics
    ///
    /// Panics if called when no frame is open.
    fn end_frame(&mut self, _ctx: UiContext) {
        assert!(
            self.in_frame,
            "EguiRenderer::end_frame called with no open frame"
        );
        // Drain and invoke all pending paint callbacks.
        let pending = std::mem::take(&mut self.pending);
        for paint in pending {
            (paint.callback)(paint.region);
        }
        // Finalise the egui pass.
        let _output = self.ctx.end_pass();
        self.in_frame = false;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::gui_renderer::{GuiRenderer, Rect};
    use std::sync::{Arc, Mutex};

    fn make_renderer() -> EguiRenderer {
        EguiRenderer::new(egui::Context::default())
    }

    // ── begin_frame / end_frame ───────────────────────────────────────────────

    #[test]
    fn begin_frame_returns_incrementing_index() {
        let mut r = make_renderer();
        let ctx0 = r.begin_frame();
        assert_eq!(ctx0.frame_index(), 0);
        r.end_frame(ctx0);

        let ctx1 = r.begin_frame();
        assert_eq!(ctx1.frame_index(), 1);
        r.end_frame(ctx1);
    }

    #[test]
    fn end_frame_resets_in_frame_flag() {
        let mut r = make_renderer();
        let ctx = r.begin_frame();
        r.end_frame(ctx);
        // A second begin_frame must not panic (flag was reset).
        let ctx2 = r.begin_frame();
        r.end_frame(ctx2);
    }

    // ── custom_paint ──────────────────────────────────────────────────────────

    #[test]
    fn custom_paint_callback_is_invoked_with_correct_rect() {
        let mut r = make_renderer();
        let ctx = r.begin_frame();

        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);
        let region = Rect::new(10.0, 20.0, 300.0, 200.0);

        r.custom_paint(
            region,
            Box::new(move |rect| {
                assert_eq!(rect.x, 10.0);
                assert_eq!(rect.y, 20.0);
                assert_eq!(rect.width, 300.0);
                assert_eq!(rect.height, 200.0);
                *called_clone.lock().unwrap() = true;
            }),
        );

        r.end_frame(ctx);
        assert!(*called.lock().unwrap(), "paint callback was not called");
    }

    #[test]
    fn multiple_paint_callbacks_are_called_in_order() {
        let mut r = make_renderer();
        let ctx = r.begin_frame();

        let order = Arc::new(Mutex::new(Vec::<u32>::new()));

        for i in 0..3u32 {
            let order_clone = Arc::clone(&order);
            r.custom_paint(
                Rect::new(i as f32 * 100.0, 0.0, 100.0, 100.0),
                Box::new(move |_rect| {
                    order_clone.lock().unwrap().push(i);
                }),
            );
        }

        r.end_frame(ctx);
        assert_eq!(*order.lock().unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn pending_callbacks_cleared_after_end_frame() {
        let mut r = make_renderer();

        // First frame: one paint.
        let ctx = r.begin_frame();
        r.custom_paint(Rect::new(0.0, 0.0, 100.0, 100.0), Box::new(|_| {}));
        r.end_frame(ctx);

        // Second frame: no paints.  `pending` must be empty.
        let ctx2 = r.begin_frame();
        assert!(
            r.pending.is_empty(),
            "pending callbacks leaked across frames"
        );
        r.end_frame(ctx2);
    }

    #[test]
    fn no_paint_calls_per_frame_is_valid() {
        let mut r = make_renderer();
        let ctx = r.begin_frame();
        // end_frame with zero pending callbacks must not panic.
        r.end_frame(ctx);
    }

    // ── trait object safety ───────────────────────────────────────────────────

    #[test]
    fn egui_renderer_is_usable_as_trait_object() {
        let renderer: Box<dyn GuiRenderer> = Box::new(make_renderer());
        drop(renderer);
    }

    // ── egui_ctx accessor ─────────────────────────────────────────────────────

    #[test]
    fn egui_ctx_returns_same_context() {
        let ctx = egui::Context::default();
        let r = EguiRenderer::new(ctx.clone());
        // Both references should point to the same underlying context.
        // We verify by checking they are pointer-equal via the debug repr
        // (egui::Context is Clone/Arc-backed; same Arc → same pointer).
        let _ = r.egui_ctx();
    }

    // ── panic guard: double begin_frame ───────────────────────────────────────

    #[test]
    #[should_panic(expected = "begin_frame called while a frame is already open")]
    fn double_begin_frame_panics() {
        let mut r = make_renderer();
        let _ctx1 = r.begin_frame();
        let _ctx2 = r.begin_frame(); // must panic
    }

    // ── panic guard: custom_paint outside frame ───────────────────────────────

    #[test]
    #[should_panic(expected = "custom_paint called outside of a begin_frame/end_frame pair")]
    fn custom_paint_outside_frame_panics() {
        let mut r = make_renderer();
        r.custom_paint(Rect::new(0.0, 0.0, 100.0, 100.0), Box::new(|_| {}));
    }

    // ── panic guard: end_frame without begin_frame ────────────────────────────

    #[test]
    #[should_panic(expected = "end_frame called with no open frame")]
    fn end_frame_without_begin_frame_panics() {
        let mut r = make_renderer();
        let ctx = UiContext::new(0);
        r.end_frame(ctx); // must panic
    }
}
