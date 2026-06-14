// path: src/modulation/lfo_waveform.rs

/// Waveform shape for an LFO.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LfoWaveform {
    /// Sinusoidal wave (smooth, continuous).
    Sine,
    /// Triangle wave (linear rise and fall).
    Triangle,
    /// Square wave (50% duty cycle).
    Square,
    /// Sawtooth wave (linear rise, instant fall).
    Sawtooth,
    /// Reverse sawtooth (instant rise, linear fall).
    ReverseSawtooth,
    /// Sample-and-hold random steps.
    SampleAndHold,
}
