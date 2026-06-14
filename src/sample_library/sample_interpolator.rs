// path: src/sample_library/sample_interpolator.rs
//
// SampleInterpolator — pitch-shifted sample reading with multiple quality modes.
//
// Design notes
// ------------
//   • All reads are allocation-free and lock-free; the interpolator may be
//     called directly from the audio thread.
//   • The pitch ratio (playback_rate / sample_rate) is computed once per voice
//     by the caller and supplied as `pitch_ratio`.  A ratio > 1.0 plays faster
//     (higher pitch); a ratio < 1.0 plays slower (lower pitch).
//   • For Sinc we use a simple windowed-sinc kernel (Hann window, 8-tap).  This
//     is intentionally not a production-quality resampler but demonstrates the
//     lock-free, alloc-free structure without pulling in a third-party crate.

use crate::sample_library::interpolation_mode::InterpolationMode;

/// Reads a single mono or de-interleaved sample at a fractional frame
/// position using the requested interpolation algorithm.
///
/// # Arguments
///
/// * `data`    – PCM frames (mono channel, or a single extracted channel slice).
/// * `pos`     – Fractional frame read-head position.
/// * `mode`    – Interpolation quality.
///
/// Returns 0.0 when `pos` is out of bounds for all modes.
#[inline]
pub fn read_sample(data: &[f32], pos: f64, mode: InterpolationMode) -> f32 {
    match mode {
        InterpolationMode::Nearest => read_nearest(data, pos),
        InterpolationMode::Linear => read_linear(data, pos),
        InterpolationMode::Cubic => read_cubic(data, pos),
        InterpolationMode::Sinc => read_sinc(data, pos),
    }
}

/// Nearest-neighbour (no interpolation).
#[inline]
fn read_nearest(data: &[f32], pos: f64) -> f32 {
    let idx = pos.round() as isize;
    if idx < 0 || idx as usize >= data.len() {
        return 0.0;
    }
    data[idx as usize]
}

/// Linear interpolation between adjacent samples.
#[inline]
fn read_linear(data: &[f32], pos: f64) -> f32 {
    let i = pos.floor() as isize;
    let frac = (pos - pos.floor()) as f32;
    let s0 = get(data, i);
    let s1 = get(data, i + 1);
    s0 + frac * (s1 - s0)
}

/// Cubic (Hermite) interpolation across four surrounding samples.
#[inline]
fn read_cubic(data: &[f32], pos: f64) -> f32 {
    let i = pos.floor() as isize;
    let t = (pos - pos.floor()) as f32;

    let p0 = get(data, i - 1);
    let p1 = get(data, i);
    let p2 = get(data, i + 1);
    let p3 = get(data, i + 2);

    // Catmull-Rom spline (zero-derivative Hermite)
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    let d = p1;

    ((a * t + b) * t + c) * t + d
}

/// Windowed-sinc interpolation (Hann window, 8-tap).
///
/// Allocation-free: uses a fixed-size stack buffer for the kernel.
#[inline]
fn read_sinc(data: &[f32], pos: f64) -> f32 {
    const TAPS: isize = 8;
    const HALF: isize = TAPS / 2;

    let i = pos.floor() as isize;
    let frac = pos - pos.floor();

    let mut out = 0.0f64;
    for k in -HALF + 1..=HALF {
        let x = (k as f64) - frac;
        let w = hann_sinc(x, TAPS as f64);
        out += w * (get(data, i + k) as f64);
    }
    // clamp to f32 range to avoid any floating-point overflow
    out.clamp(f32::MIN as f64, f32::MAX as f64) as f32
}

/// Hann-windowed sinc function evaluated at `x` with window width `n`.
#[inline]
fn hann_sinc(x: f64, n: f64) -> f64 {
    if x.abs() < f64::EPSILON {
        return 1.0;
    }
    let sinc = (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x);
    // Hann window: 0.5 * (1 + cos(2π x / n))
    let window = 0.5 * (1.0 + (2.0 * std::f64::consts::PI * x / n).cos());
    sinc * window
}

/// Safe sample lookup — returns 0.0 for out-of-bounds indices.
#[inline]
fn get(data: &[f32], idx: isize) -> f32 {
    if idx < 0 || idx as usize >= data.len() {
        0.0
    } else {
        data[idx as usize]
    }
}

/// A stateful, pitch-shifting sample reader for a single voice.
///
/// `SampleInterpolator` tracks a fractional read-head position and advances
/// it by `pitch_ratio` on each call to [`SampleInterpolator::next_frame`].
///
/// # Audio-thread safety
///
/// * No allocation — uses only stack memory.
/// * No locks.
/// * No I/O.
///
/// # Example
///
/// ```
/// use crest_synth::sample_library::interpolation_mode::InterpolationMode;
/// use crest_synth::sample_library::sample_interpolator::SampleInterpolator;
///
/// let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
/// let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.5);
/// let sample = interp.next_frame(&data);
/// assert!(sample.is_finite());
/// ```
#[derive(Debug, Clone)]
pub struct SampleInterpolator {
    /// Interpolation algorithm.
    mode: InterpolationMode,
    /// How many source frames to advance per output frame.
    pitch_ratio: f64,
    /// Current fractional read-head position in source frames.
    position: f64,
}

impl SampleInterpolator {
    /// Construct a new `SampleInterpolator`.
    ///
    /// # Arguments
    ///
    /// * `mode`        – Interpolation quality.
    /// * `pitch_ratio` – Source frames consumed per output frame.
    ///                   1.0 = original pitch; 2.0 = one octave up; 0.5 = one octave down.
    pub fn new(mode: InterpolationMode, pitch_ratio: f64) -> Self {
        Self {
            mode,
            pitch_ratio,
            position: 0.0,
        }
    }

    /// Read the next output sample and advance the read-head.
    ///
    /// Returns 0.0 when the read-head is past the end of `data`.
    ///
    /// This method is allocation-free and lock-free and may be called from
    /// the audio thread.
    #[inline]
    pub fn next_frame(&mut self, data: &[f32]) -> f32 {
        let out = read_sample(data, self.position, self.mode);
        self.position += self.pitch_ratio;
        out
    }

    /// Returns `true` when the read-head has passed the end of a sample
    /// buffer of length `len`.
    #[inline]
    pub fn is_finished(&self, len: usize) -> bool {
        self.position >= len as f64
    }

    /// Reset the read-head to the start of the sample.
    pub fn reset(&mut self) {
        self.position = 0.0;
    }

    /// Seek to an arbitrary fractional frame position.
    pub fn seek(&mut self, position: f64) {
        self.position = position;
    }

    /// Current fractional read-head position.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Active interpolation mode.
    pub fn mode(&self) -> InterpolationMode {
        self.mode
    }

    /// Current pitch ratio.
    pub fn pitch_ratio(&self) -> f64 {
        self.pitch_ratio
    }

    /// Update the pitch ratio (e.g., after a pitch bend event).
    ///
    /// This is called from the audio thread and must not allocate.
    pub fn set_pitch_ratio(&mut self, pitch_ratio: f64) {
        self.pitch_ratio = pitch_ratio;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple ascending ramp: data[i] = i as f32.
    fn ramp(len: usize) -> Vec<f32> {
        (0..len).map(|i| i as f32).collect()
    }

    // ── read_sample ────────────────────────────────────────────────────────

    #[test]
    fn sample_interpolator_nearest_integer_pos() {
        let data = ramp(8);
        // At exact integer positions nearest == the value itself
        assert_eq!(read_sample(&data, 0.0, InterpolationMode::Nearest), 0.0);
        assert_eq!(read_sample(&data, 3.0, InterpolationMode::Nearest), 3.0);
        assert_eq!(read_sample(&data, 7.0, InterpolationMode::Nearest), 7.0);
    }

    #[test]
    fn sample_interpolator_nearest_out_of_bounds_returns_zero() {
        let data = ramp(8);
        assert_eq!(read_sample(&data, -1.0, InterpolationMode::Nearest), 0.0);
        assert_eq!(read_sample(&data, 8.0, InterpolationMode::Nearest), 0.0);
        assert_eq!(read_sample(&data, 100.0, InterpolationMode::Nearest), 0.0);
    }

    #[test]
    fn sample_interpolator_nearest_rounds_to_nearest() {
        let data = ramp(8);
        // 2.4 rounds to 2, 2.6 rounds to 3
        assert_eq!(read_sample(&data, 2.4, InterpolationMode::Nearest), 2.0);
        assert_eq!(read_sample(&data, 2.6, InterpolationMode::Nearest), 3.0);
    }

    #[test]
    fn sample_interpolator_linear_integer_pos_exact() {
        let data = ramp(8);
        assert_eq!(read_sample(&data, 4.0, InterpolationMode::Linear), 4.0);
    }

    #[test]
    fn sample_interpolator_linear_midpoint_is_average() {
        let data = ramp(8);
        // Midpoint between 3 and 4 must be 3.5
        let v = read_sample(&data, 3.5, InterpolationMode::Linear);
        assert!((v - 3.5).abs() < 1e-5, "expected 3.5, got {v}");
    }

    #[test]
    fn sample_interpolator_linear_out_of_bounds_returns_zero() {
        let data = ramp(8);
        assert_eq!(read_sample(&data, -1.0, InterpolationMode::Linear), 0.0);
    }

    #[test]
    fn sample_interpolator_cubic_integer_pos_exact() {
        let data = ramp(8);
        // At an exact integer position cubic should reproduce the value
        let v = read_sample(&data, 4.0, InterpolationMode::Cubic);
        assert!((v - 4.0).abs() < 1e-4, "expected 4.0, got {v}");
    }

    #[test]
    fn sample_interpolator_cubic_ramp_midpoint_is_smooth() {
        let data = ramp(8);
        // For a linear ramp any interpolator should give ≈ the true value
        let v = read_sample(&data, 3.5, InterpolationMode::Cubic);
        assert!((v - 3.5).abs() < 0.01, "expected ~3.5, got {v}");
    }

    #[test]
    fn sample_interpolator_sinc_integer_pos_exact() {
        let data = ramp(16);
        let v = read_sample(&data, 8.0, InterpolationMode::Sinc);
        assert!((v - 8.0).abs() < 0.1, "expected ~8.0, got {v}");
    }

    #[test]
    fn sample_interpolator_sinc_output_is_finite() {
        let data = ramp(16);
        let v = read_sample(&data, 7.5, InterpolationMode::Sinc);
        assert!(v.is_finite());
    }

    #[test]
    fn sample_interpolator_sinc_ramp_midpoint_reasonable() {
        let data = ramp(16);
        // For a linear ramp sinc should give a result close to the true value
        let v = read_sample(&data, 7.5, InterpolationMode::Sinc);
        assert!((v - 7.5).abs() < 0.5, "expected ~7.5, got {v}");
    }

    // ── SampleInterpolator ─────────────────────────────────────────────────

    #[test]
    fn sample_interpolator_new_starts_at_zero() {
        let interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        assert!((interp.position() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_mode_and_ratio_accessors() {
        let interp = SampleInterpolator::new(InterpolationMode::Cubic, 1.5);
        assert_eq!(interp.mode(), InterpolationMode::Cubic);
        assert!((interp.pitch_ratio() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_next_frame_advances_position() {
        let data = ramp(16);
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 2.0);
        let _ = interp.next_frame(&data);
        assert!((interp.position() - 2.0).abs() < f64::EPSILON);
        let _ = interp.next_frame(&data);
        assert!((interp.position() - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_next_frame_returns_correct_value() {
        let data = ramp(16);
        let mut interp = SampleInterpolator::new(InterpolationMode::Nearest, 1.0);
        let v = interp.next_frame(&data);
        assert!((v - 0.0).abs() < f32::EPSILON);
        let v2 = interp.next_frame(&data);
        assert!((v2 - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sample_interpolator_is_finished_false_when_in_bounds() {
        let interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        assert!(!interp.is_finished(16));
    }

    #[test]
    fn sample_interpolator_is_finished_true_when_past_end() {
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        interp.seek(16.0);
        assert!(interp.is_finished(16));
    }

    #[test]
    fn sample_interpolator_reset_returns_to_zero() {
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        interp.seek(8.0);
        interp.reset();
        assert!((interp.position() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_seek_sets_position() {
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        interp.seek(3.75);
        assert!((interp.position() - 3.75).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_set_pitch_ratio_updates() {
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        interp.set_pitch_ratio(0.5);
        assert!((interp.pitch_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_pitch_up_reads_fewer_output_frames() {
        let data = ramp(32);
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 2.0);
        let mut count = 0;
        while !interp.is_finished(data.len()) {
            let _ = interp.next_frame(&data);
            count += 1;
        }
        // Pitch ratio 2.0 => half as many output frames as source frames
        assert_eq!(count, 16);
    }

    #[test]
    fn sample_interpolator_pitch_unity_reads_all_frames() {
        let data = ramp(16);
        let mut interp = SampleInterpolator::new(InterpolationMode::Nearest, 1.0);
        let mut count = 0;
        while !interp.is_finished(data.len()) {
            let _ = interp.next_frame(&data);
            count += 1;
        }
        assert_eq!(count, 16);
    }

    #[test]
    fn sample_interpolator_clone_is_independent() {
        let data = ramp(16);
        let mut original = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        let _ = original.next_frame(&data);
        let cloned = original.clone();
        // Both should have the same position
        assert!((original.position() - cloned.position()).abs() < f64::EPSILON);
        // Advancing the original should not affect the clone
        let mut orig2 = original;
        let _ = orig2.next_frame(&data);
        assert!((cloned.position() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_interpolator_empty_data_returns_zero() {
        let data: Vec<f32> = vec![];
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 1.0);
        assert_eq!(interp.next_frame(&data), 0.0);
    }

    #[test]
    fn sample_interpolator_all_modes_produce_finite_output() {
        let data: Vec<f32> = (0..32).map(|i| (i as f32) * 0.1).collect();
        for mode in [
            InterpolationMode::Nearest,
            InterpolationMode::Linear,
            InterpolationMode::Cubic,
            InterpolationMode::Sinc,
        ] {
            let mut interp = SampleInterpolator::new(mode, 1.0);
            while !interp.is_finished(data.len()) {
                let v = interp.next_frame(&data);
                assert!(v.is_finite(), "mode {mode:?} produced non-finite value {v}");
            }
        }
    }

    #[test]
    fn sample_interpolator_fractional_pitch_down() {
        // pitch_ratio=0.5 → two output frames per source frame
        let data: Vec<f32> = vec![0.0, 1.0];
        let mut interp = SampleInterpolator::new(InterpolationMode::Linear, 0.5);
        // 4 output frames before position reaches 2.0
        let frames: Vec<f32> = (0..4).map(|_| interp.next_frame(&data)).collect();
        assert_eq!(frames.len(), 4);
        for f in &frames {
            assert!(f.is_finite());
        }
    }
}
