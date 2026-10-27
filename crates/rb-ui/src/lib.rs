//! The bundled desktop UI (milestone m-3): the song window of the original, drawn from the
//! SVG art set `assets/ui` with egui, with the original's widget behavior
//! (docs/ui_spec.md). `model` holds the state and the events the engine must hear (wired by
//! task-045); `behavior` is the widgets' interaction math; `app` is the window.

pub mod app;
pub mod art;
pub mod behavior;
pub mod layout;
pub mod model;
