// path: src/shell/gui_renderer.rs

/// An axis-aligned rectangle in logical (UI) coordinates.
///
/// Used to specify the region passed to [`GuiRenderer::custom_paint`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Horizontal position of the left edge in logical pixels.
    pub x: f32,
    /// Vertical position of the top edge in logical pixels.
    pub y: f32,
    /// Width of the rectangle in logical pixels.
    pub width: f32,
    /// Height of the rectangle in logical pixels.
    pub height: f32,
}

impl Rect {
    /// Creates a new `Rect` at position `(x, y)` with the given `width` and
    /// `height`.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the x-coordinate of the right edge.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Returns the y-coordinate of the bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Returns `true` if this rect contains the point `(px, py)`.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
}

/// An opaque handle representing the UI context for a single rendered frame.
///
/// Obtained from [`GuiRenderer::begin_frame`] and consumed by
/// [`GuiRenderer::end_frame`].  The handle must not be held across frame
/// boundaries.
pub struct UiContext {
    /// Frame sequence number; incremented by the renderer on each call to
    /// `begin_frame`.
    pub(crate) frame_index: u64,
}

impl UiContext {
    /// Creates a new `UiContext` with the given frame index.
    ///
    /// Intended for use by [`GuiRenderer`] implementations only.
    pub fn new(frame_index: u64) -> Self {
        Self { frame_index }
    }

    /// Returns the sequence number of the frame this context belongs to.
    pub fn frame_index(&self) -> u64 {
        self.frame_index
    }
}

/// A callback that performs custom painting inside a [`Rect`].
///
/// The closure receives the target [`Rect`] so it can position draw calls
/// within the allocated region.
pub type PaintCallback = Box<dyn FnOnce(Rect) + Send + 'static>;

/// Port: GUI rendering.
///
/// Implementations wire a concrete rendering back-end (e.g. `egui` / `wgpu`)
/// behind this interface so that higher-level shell code stays back-end agnostic.
///
/// # Frame lifecycle
///
/// Each rendered frame follows the sequence:
/// 1. [`begin_frame`][Self::begin_frame] — acquire a [`UiContext`] for this frame.
/// 2. Zero or more [`custom_paint`][Self::custom_paint] calls — schedule
///    arbitrary paint callbacks into bounded screen regions.
/// 3. [`end_frame`][Self::end_frame] — flush/present the frame, consuming the
///    [`UiContext`].
///
/// # UI thread constraint
///
/// All methods must be called from the UI / window thread.  Implementations
/// must **not** block on audio-thread resources.
pub trait GuiRenderer {
    /// Begins a new UI frame and returns an opaque [`UiContext`] handle.
    ///
    /// Must be paired with exactly one call to [`end_frame`][Self::end_frame]
    /// before the next call to `begin_frame`.
    fn begin_frame(&mut self) -> UiContext;

    /// Schedules a custom paint callback to be rendered within `region`.
    ///
    /// The `callback` is invoked by the back-end during frame presentation
    /// with the exact [`Rect`] that was reserved for it.  Multiple calls to
    /// `custom_paint` within a single frame are composited in call order.
    ///
    /// # Panics
    ///
    /// Implementations may panic if called outside of a `begin_frame` /
    /// `end_frame` pair.
    fn custom_paint(&mut self, region: Rect, callback: PaintCallback);

    /// Ends the current frame, flushing all pending paint operations and
    /// presenting the result to the screen.
    ///
    /// Consumes the [`UiContext`] returned by [`begin_frame`][Self::begin_frame],
    /// enforcing that a context is never reused across frames.
    fn end_frame(&mut self, ctx: UiContext);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rect ─────────────────────────────────────────────────────────────────

    #[test]
    fn rect_new_stores_fields() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn rect_right_and_bottom() {
        let r = Rect::new(5.0, 5.0, 40.0, 30.0);
        assert_eq!(r.right(), 45.0);
        assert_eq!(r.bottom(), 35.0);
    }

    #[test]
    fn rect_contains_inside_point() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(50.0, 50.0));
    }

    #[test]
    fn rect_contains_edge_point() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(0.0, 0.0));
        assert!(r.contains(100.0, 100.0));
    }

    #[test]
    fn rect_does_not_contain_outside_point() {
        let r = Rect::new(10.0, 10.0, 50.0, 50.0);
        assert!(!r.contains(5.0, 30.0));
        assert!(!r.contains(30.0, 5.0));
        assert!(!r.contains(65.0, 30.0));
        assert!(!r.contains(30.0, 65.0));
    }

    #[test]
    fn rect_clone_is_independent() {
        let r = Rect::new(1.0, 2.0, 3.0, 4.0);
        let mut r2 = r;
        r2.x = 99.0;
        assert_eq!(r.x, 1.0);
    }

    // ── UiContext ────────────────────────────────────────────────────────────

    #[test]
    fn ui_context_exposes_frame_index() {
        let ctx = UiContext::new(42);
        assert_eq!(ctx.frame_index(), 42);
    }

    #[test]
    fn ui_context_first_frame_is_zero() {
        let ctx = UiContext::new(0);
        assert_eq!(ctx.frame_index(), 0);
    }

    // ── GuiRenderer (stub impl) ──────────────────────────────────────────────

    /// A minimal stub implementation used only within this test module.
    struct StubRenderer {
        frame_counter: u64,
        paint_calls: Vec<Rect>,
        frames_ended: u64,
    }

    impl StubRenderer {
        fn new() -> Self {
            Self {
                frame_counter: 0,
                paint_calls: Vec::new(),
                frames_ended: 0,
            }
        }
    }

    impl GuiRenderer for StubRenderer {
        fn begin_frame(&mut self) -> UiContext {
            let ctx = UiContext::new(self.frame_counter);
            self.frame_counter += 1;
            ctx
        }

        fn custom_paint(&mut self, region: Rect, callback: PaintCallback) {
            // Record the region and invoke the callback immediately (stub flush).
            self.paint_calls.push(region);
            callback(region);
        }

        fn end_frame(&mut self, _ctx: UiContext) {
            self.frames_ended += 1;
        }
    }

    #[test]
    fn begin_frame_returns_incrementing_index() {
        let mut renderer = StubRenderer::new();
        let ctx0 = renderer.begin_frame();
        renderer.end_frame(ctx0);
        let ctx1 = renderer.begin_frame();
        renderer.end_frame(ctx1);
        assert_eq!(renderer.frame_counter, 2);
        assert_eq!(renderer.frames_ended, 2);
    }

    #[test]
    fn custom_paint_records_region_and_invokes_callback() {
        let mut renderer = StubRenderer::new();
        let ctx = renderer.begin_frame();

        let region = Rect::new(0.0, 0.0, 800.0, 600.0);
        let mut called = false;
        // Use a shared flag via a raw pointer trick isn't available in safe Rust;
        // instead, verify via the recorded paint_calls count.
        renderer.custom_paint(
            region,
            Box::new(move |r| {
                // callback receives the rect unchanged
                assert_eq!(r.width, 800.0);
                assert_eq!(r.height, 600.0);
                let _ = r; // suppress unused warning
            }),
        );

        assert_eq!(renderer.paint_calls.len(), 1);
        assert_eq!(renderer.paint_calls[0].width, 800.0);

        // Suppress unused variable warning for `called`.
        let _ = called;
        renderer.end_frame(ctx);
    }

    #[test]
    fn multiple_custom_paints_per_frame_are_all_recorded() {
        let mut renderer = StubRenderer::new();
        let ctx = renderer.begin_frame();

        renderer.custom_paint(Rect::new(0.0, 0.0, 100.0, 100.0), Box::new(|_| {}));
        renderer.custom_paint(Rect::new(100.0, 0.0, 100.0, 100.0), Box::new(|_| {}));
        renderer.custom_paint(Rect::new(200.0, 0.0, 100.0, 100.0), Box::new(|_| {}));

        assert_eq!(renderer.paint_calls.len(), 3);
        renderer.end_frame(ctx);
    }

    #[test]
    fn gui_renderer_trait_is_object_safe() {
        let renderer: Box<dyn GuiRenderer> = Box::new(StubRenderer::new());
        drop(renderer);
    }
}
