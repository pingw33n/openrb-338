---
id: m-1
title: "m-1: Audio engine CLI (.rbs to FLAC/Ogg)"
---

## Description

The Rust audio engine reimplementing ReBirth.dll, packaged as a CLI that takes an .rbs file and writes the rendered song (the song loop, as the original export) to .flac or .ogg (and .wav for comparison). Built-in drum samples only. Done when every reference of the m-0 corpus matches the engine's render (byte-exact, or within the tolerance a Decide task fixed), checked by cargo test.
Background: docs/initial_investigation.md (sections 4 to 9, 11).
