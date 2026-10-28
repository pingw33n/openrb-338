//! The bundled desktop UI (milestone m-3): the song window of the original, drawn from the
//! SVG art set `assets/ui` with egui, with the original's widget behavior
//! (docs/ui_spec.md). `model` holds the state and the events the engine must hear; `link`
//! turns them into the original panel's messages and the real-time player's commands and
//! takes the player's snapshot back into the panel (task-045); `session` opens songs and the
//! built-in kit; `behavior` is the widgets' interaction math; `app` is the window.

pub mod app;
pub mod art;
pub mod behavior;
pub mod browser;
pub mod document;
pub mod info;
pub mod keys;
pub mod layout;
pub mod link;
pub mod menu;
pub mod model;
pub mod session;
pub mod settings;
pub mod shortcuts;
