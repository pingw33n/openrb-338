//! The library behind the `orb` command line tool: what `orb render`, `orb compare` and `orb play`
//! share, and what the UI's menu (task-047) reuses so that its export writes the file `orb render`
//! writes: the song loader, the mod lookup, the renderer and the encoders.

pub mod cd;
pub mod encode;
pub mod info;
pub mod mods;
pub mod recrypt;
pub mod render;

/// A failed command: the exit code (1 the song or the output failed, 2 a usage error) and the
/// message.
#[derive(Debug)]
pub struct Fail(pub u8, pub String);

pub fn fail(code: u8, msg: impl Into<String>) -> Fail {
    Fail(code, msg.into())
}
