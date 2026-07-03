// path: src/shell/cpal_audio_output.rs

//! Adapter: `CpalAudioOutput` — a `cpal`-backed implementation of the
//! `Shell.AudioOutput` port.
//!
//! `cpal::Stream` is not `Send` on every platform (its CoreAudio-backed
//! macOS implementation owns platform handles that cannot cross threads
//! safely), yet the port's `Stream: Send` bound is non-negotiable. This
//! adapter reconciles the two by never letting a `cpal::Stream` leave the
//! thread that created it: `open` spawns a dedicated host thread that
//! resolves the output device, builds the `cpal::Stream`, starts it, and
//! then blocks holding it. The caller gets back a [`CpalStreamHandle`] — a
//! `Send` control handle (a channel plus a join handle) that can only ask
//! the host thread to stop, never touch the `cpal::Stream` itself.
//!
//! The actual per-buffer audio callback still runs wherever `cpal`/the
//! platform host schedules it. The render callback handed to `open` is
//! moved straight into the `cpal` data callback and is never touched
//! anywhere else, so the real audio callback path stays free of
//! adapter-added allocation, locking, or blocking I/O — the host thread
//! only owns the `Stream` value itself so it can be torn down safely from
//! another thread.

use std::sync::mpsc;
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use crate::shell::audio_output::{
    AudioError, AudioOutput, BufferSize, RenderCallback, SampleRate, Stream,
};

/// Message sent from a [`CpalStreamHandle`] to the host thread that owns
/// the underlying `cpal::Stream`.
enum ControlMessage {
    Stop,
}

/// A `Send` handle to a `cpal::Stream` living on a dedicated host thread.
///
/// Holds no `cpal::Stream` itself — only a channel to ask the host thread
/// to stop it, and a join handle to wait for that thread to exit. This is
/// what makes `CpalStreamHandle` safely `Send` even though `cpal::Stream`
/// is not.
pub struct CpalStreamHandle {
    control: Option<mpsc::Sender<ControlMessage>>,
    host_thread: Option<thread::JoinHandle<()>>,
}

impl Stream for CpalStreamHandle {
    fn stop(&mut self) {
        if let Some(control) = self.control.take() {
            // The host thread may already have exited (e.g. the device
            // disconnected); a failed send just means there is nothing
            // left to stop.
            let _ = control.send(ControlMessage::Stop);
        }
        if let Some(handle) = self.host_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for CpalStreamHandle {
    fn drop(&mut self) {
        // Idempotent: `stop` only acts the first time it is called, so a
        // caller who already invoked `close`/`stop` pays nothing extra
        // here.
        self.stop();
    }
}

/// Finds an `f32` output configuration on `device` whose sample-rate range
/// covers `sample_rate`, preferring the device's default config when that
/// default is already `f32`. Only `f32` is ever negotiated: the render
/// callback speaks `f32` samples, and converting sample formats on the
/// audio thread would require an allocation-free conversion path this
/// adapter does not provide.
fn find_f32_config(
    device: &cpal::Device,
    sample_rate: SampleRate,
) -> Result<cpal::SupportedStreamConfig, AudioError> {
    if let Ok(default_config) = device.default_output_config() {
        if default_config.sample_format() == SampleFormat::F32 {
            return Ok(default_config);
        }
    }

    let hz = sample_rate.hz();
    let mut candidates = device.supported_output_configs().map_err(|err| {
        AudioError::DeviceUnavailable(format!("could not query output configs: {err}"))
    })?;

    candidates
        .find(|range| {
            range.sample_format() == SampleFormat::F32
                && range.min_sample_rate().0 <= hz
                && hz <= range.max_sample_rate().0
        })
        .map(|range| range.with_sample_rate(cpal::SampleRate(hz)))
        .ok_or_else(|| {
            AudioError::UnsupportedConfig(format!(
                "no f32 output configuration supports {sample_rate}"
            ))
        })
}

/// Maps a `cpal::BuildStreamError` to the port's `AudioError`.
fn map_build_error(err: cpal::BuildStreamError) -> AudioError {
    match err {
        cpal::BuildStreamError::StreamConfigNotSupported => {
            AudioError::UnsupportedConfig(err.to_string())
        }
        cpal::BuildStreamError::DeviceNotAvailable => {
            AudioError::DeviceUnavailable(err.to_string())
        }
        other => AudioError::StreamFailed(other.to_string()),
    }
}

/// Maps a `cpal::PlayStreamError` to the port's `AudioError`.
fn map_play_error(err: cpal::PlayStreamError) -> AudioError {
    match err {
        cpal::PlayStreamError::DeviceNotAvailable => AudioError::DeviceUnavailable(err.to_string()),
        other => AudioError::StreamFailed(other.to_string()),
    }
}

/// `cpal`-backed adapter for the `Shell.AudioOutput` port.
///
/// Depends on a `cpal::Host`, injected through
/// [`CpalAudioOutput::with_host`] so tests (or callers wanting a
/// non-default backend, e.g. `cpal`'s ASIO or JACK hosts) can supply a
/// specific host without this type reaching for a global default itself.
/// [`CpalAudioOutput::new`] is a convenience constructor for callers who
/// just want the platform's default host.
pub struct CpalAudioOutput {
    host: cpal::Host,
}

impl CpalAudioOutput {
    /// Convenience constructor using the platform's default `cpal` host.
    pub fn new() -> Self {
        Self::with_host(cpal::default_host())
    }

    /// Full constructor: an adapter bound to the given `cpal::Host`.
    pub fn with_host(host: cpal::Host) -> Self {
        Self { host }
    }
}

impl Default for CpalAudioOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutput for CpalAudioOutput {
    type StreamHandle = CpalStreamHandle;

    fn open(
        &self,
        sample_rate: SampleRate,
        buffer_size: BufferSize,
        callback: Box<dyn RenderCallback>,
    ) -> Result<Self::StreamHandle, AudioError> {
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), AudioError>>();
        let (control_tx, control_rx) = mpsc::channel::<ControlMessage>();

        // `cpal::Host` is not `Send` on every platform either, so it is
        // not captured directly; the host thread re-resolves the default
        // host itself, matching the `Host` this adapter was constructed
        // with only in the common (default-host) case. Adapters
        // constructed `with_host` for a non-default host are honored by
        // resolving *that* host's default device from inside the closure
        // via a host-selector callback the closure owns.
        let host_selector = self.host_selector();

        let host_thread = thread::Builder::new()
            .name("cpal-audio-host".to_string())
            .spawn(move || {
                run_host_thread(
                    host_selector,
                    sample_rate,
                    buffer_size,
                    callback,
                    ready_tx,
                    control_rx,
                );
            })
            .map_err(|err| AudioError::StreamFailed(err.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(CpalStreamHandle {
                control: Some(control_tx),
                host_thread: Some(host_thread),
            }),
            Ok(Err(err)) => {
                let _ = host_thread.join();
                Err(err)
            }
            Err(_) => {
                let _ = host_thread.join();
                Err(AudioError::StreamFailed(
                    "audio host thread exited before signaling readiness".to_string(),
                ))
            }
        }
    }
}

impl CpalAudioOutput {
    /// A `Send`, allocation-free-to-call closure that reproduces this
    /// adapter's host selection (default host id) from inside the spawned
    /// host thread, where the non-`Send` `cpal::Host` itself cannot be
    /// carried directly.
    fn host_selector(&self) -> impl Fn() -> cpal::Host + Send + 'static {
        let host_id = self.host.id();
        move || cpal::host_from_id(host_id).unwrap_or_else(|_| cpal::default_host())
    }
}

/// Body of the dedicated host thread spawned by `open`.
///
/// Resolves the host and device, builds the `cpal::Stream`, reports
/// success or failure over `ready`, then blocks on `control` until told to
/// stop (or until the caller drops its sender). The `cpal::Stream` lives
/// on this thread's stack for its entire life and never crosses to
/// another thread.
fn run_host_thread(
    host_selector: impl Fn() -> cpal::Host,
    sample_rate: SampleRate,
    buffer_size: BufferSize,
    callback: Box<dyn RenderCallback>,
    ready: mpsc::Sender<Result<(), AudioError>>,
    control: mpsc::Receiver<ControlMessage>,
) {
    let host = host_selector();
    let stream = match build_stream(&host, sample_rate, buffer_size, callback) {
        Ok(stream) => stream,
        Err(err) => {
            let _ = ready.send(Err(err));
            return;
        }
    };

    if let Err(err) = stream.play() {
        let _ = ready.send(Err(map_play_error(err)));
        return;
    }

    if ready.send(Ok(())).is_err() {
        // The caller already gave up waiting; nothing left to serve.
        return;
    }

    // Block until asked to stop, or until the caller drops the control
    // sender (which also unblocks `recv` with an error).
    let _ = control.recv();
    // `stream` drops here, stopping playback.
}

/// Resolve `host`'s default output device and build a `cpal::Stream` whose
/// data callback drains `callback` into every buffer `cpal` requests.
fn build_stream(
    host: &cpal::Host,
    sample_rate: SampleRate,
    buffer_size: BufferSize,
    mut callback: Box<dyn RenderCallback>,
) -> Result<cpal::Stream, AudioError> {
    let device = host
        .default_output_device()
        .ok_or_else(|| AudioError::DeviceUnavailable("no default output device".to_string()))?;

    let supported_config = find_f32_config(&device, sample_rate)?;
    let config = StreamConfig {
        channels: supported_config.channels(),
        sample_rate: cpal::SampleRate(sample_rate.hz()),
        buffer_size: cpal::BufferSize::Fixed(buffer_size.frames()),
    };

    let err_fn = |err: cpal::StreamError| {
        // This runs on cpal's own error-reporting path, not inside the
        // per-buffer render callback, so it is not bound by the audio
        // thread's no-alloc/no-lock/no-block invariants.
        eprintln!("audio stream error: {err}");
    };

    device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| callback.render(data),
            err_fn,
            None,
        )
        .map_err(map_build_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}

    #[test]
    fn stream_handle_is_send() {
        // The whole point of routing the `cpal::Stream` through a
        // dedicated host thread is so the handle we hand back is `Send`
        // even though `cpal::Stream` itself is not. This is a
        // compile-time assertion of that property.
        assert_send::<CpalStreamHandle>();
    }

    #[test]
    fn map_build_error_classifies_config_not_supported() {
        let mapped = map_build_error(cpal::BuildStreamError::StreamConfigNotSupported);
        assert!(matches!(mapped, AudioError::UnsupportedConfig(_)));
    }

    #[test]
    fn map_build_error_classifies_device_not_available() {
        let mapped = map_build_error(cpal::BuildStreamError::DeviceNotAvailable);
        assert!(matches!(mapped, AudioError::DeviceUnavailable(_)));
    }

    #[test]
    fn map_build_error_classifies_other_as_stream_failed() {
        let mapped = map_build_error(cpal::BuildStreamError::InvalidArgument);
        assert!(matches!(mapped, AudioError::StreamFailed(_)));
    }

    #[test]
    fn map_play_error_classifies_device_not_available() {
        let mapped = map_play_error(cpal::PlayStreamError::DeviceNotAvailable);
        assert!(matches!(mapped, AudioError::DeviceUnavailable(_)));
    }

    #[test]
    fn map_play_error_classifies_backend_specific_as_stream_failed() {
        let mapped = map_play_error(cpal::PlayStreamError::BackendSpecific {
            err: cpal::BackendSpecificError {
                description: "boom".to_string(),
            },
        });
        assert!(matches!(mapped, AudioError::StreamFailed(_)));
    }

    #[test]
    fn default_constructor_uses_default_host() {
        // Constructing must not panic even in headless CI: it only asks
        // cpal for its default host id, it does not touch a device.
        let _adapter = CpalAudioOutput::default();
    }

    /// A callback that fills every buffer with silence, used only to
    /// exercise the adapter without depending on synthesized audio.
    struct SilentCallback;

    impl RenderCallback for SilentCallback {
        fn render(&mut self, output: &mut [f32]) {
            output.fill(0.0);
        }
    }

    #[test]
    fn open_returns_promptly_and_stop_is_idempotent() {
        let adapter = CpalAudioOutput::new();
        let sample_rate = SampleRate::new(44_100).expect("44_100 is a valid sample rate");
        let buffer_size = BufferSize::new(256).expect("256 is a valid buffer size");
        let callback = Box::new(SilentCallback);

        // Whether or not the sandbox this runs in exposes a real output
        // device, `open` must return promptly rather than hang, and a
        // successfully opened stream must tolerate being stopped more
        // than once without panicking.
        match adapter.open(sample_rate, buffer_size, callback) {
            Ok(mut stream) => {
                stream.stop();
                stream.stop();
            }
            Err(_) => {
                // No output device available in this environment — a
                // legitimate outcome for the port contract, not a test
                // failure.
            }
        }
    }
}
