---
id: m-3
title: "m-3: Bundled UI (scalable SVG, original layout)"
---

## Description

A desktop UI for rb with the layout and functionality of the original ReBirth panel, redrawn as scalable SVG art of our own (not the RBM skins). It plays songs in real time through the m-1 engine: transport, pattern and song mode, the four devices, mixer and effects, pattern editing. From the original menu only: load and save a song, export to a file, choose an RBM mod, and the Vintage ReBirth Synth Sound flag. No native dialogs: file selection is an in-app browser. The Rust UI toolkit is chosen by a Decide task (egui suggested by the user). Depends on m-1 (engine) and, for the mod chooser, m-2.
