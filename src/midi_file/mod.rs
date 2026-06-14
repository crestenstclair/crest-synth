// path: src/midi_file/mod.rs

pub mod loader;

pub use loader::{load, MidiLoadError};
