# RustySynth

Source: crates.io `rustysynth` 1.3.6; MIT, see LICENSE.txt.
Original archive SHA-256: `e3ca93af923df5fc03beddbf464242620fd24daa9e10f9ecd56eb9571eb7ba38`.
Repository: https://github.com/sinshu/rustysynth
Revision: `8cc11fc0b10422adb54107757f100d3d6ae0ef96`.

Crest changes only positive prepared voice-count validation, removing the
upstream 8–256 configuration restriction. Upstream already owns a dynamic
voice vector. Audio algorithms are unchanged. All resources are prepared
before activation and retired off callback.
