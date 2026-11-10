//! The open song inside the host's plugin state.
//!
//! Live asks for the state on the thread that saves the project, not on the
//! audio thread. The bytes are the ones File > Save writes. A missing field is
//! an empty song, so a set saved before this field still opens.

use std::sync::{Arc, Mutex};

use nih_plug::params::persist::PersistentField;
use orb_ui::app::App;

/// The song blob the host persists under the stable name `rbs`.
pub struct SongBlob {
    bytes: Mutex<Vec<u8>>,
    app: Mutex<Option<Arc<Mutex<App>>>>,
}

impl SongBlob {
    pub fn new() -> Self {
        Self {
            bytes: Mutex::new(Vec::new()),
            app: Mutex::new(None),
        }
    }

    /// The panel the save reads. Called once the plugin's `App` exists.
    pub fn attach(&self, app: Arc<Mutex<App>>) {
        *self.app.lock().expect("song app") = Some(app);
    }

    /// The last composed song. Empty until a save or a state load.
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.lock().expect("song bytes").clone()
    }

    /// Stores the song File > Save would write.
    pub fn capture(&self, app: &App) {
        if let Ok(bytes) = app.song_bytes() {
            *self.bytes.lock().expect("song bytes") = bytes;
        }
    }

    /// Composes the song when the panel is free. A frame that is drawing keeps
    /// the copy it stored at the end of the previous frame.
    fn refresh(&self) {
        let app = self.app.lock().expect("song app").clone();
        let Some(app) = app else {
            return;
        };
        if let Ok(app) = app.try_lock() {
            self.capture(&app);
        }
    }
}

impl<'a> PersistentField<'a, Vec<u8>> for SongBlob {
    fn set(&self, new_value: Vec<u8>) {
        *self.bytes.lock().expect("song bytes") = new_value;
    }

    fn map<F, R>(&self, f: F) -> R
    where
        F: Fn(&Vec<u8>) -> R,
    {
        self.refresh();
        let bytes = self.bytes.lock().expect("song bytes");
        f(&bytes)
    }
}
