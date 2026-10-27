//! The 46 built-in drum samples, read at run time from the user's `ReBirth.dll` (task-016: no
//! Windows API, nothing of Propellerhead's embedded or redistributed).
//!
//! The DLL keeps them as resources of the named type `"AIFF"`, IDs 1000..=1045
//! (`docs/initial_investigation.md` section 7.4). [`SampleSet::from_dll`] finds them with a
//! small PE reader and parses each AIFF as `CAIFFReader` does: 16-bit only, channel 0 only, the
//! sample rate ignored. Each sample keeps its audio as raw `i16` followed by 512 zero samples,
//! the padding of `CWaveSample` (0x1000A9C0), which the voices may read past the end.
//!
//! The engine takes a loaded [`SampleSet`]; finding the file is the caller's job.

mod aiff;
pub mod pe;

use std::fmt;
use std::sync::Arc;

pub use aiff::PADDING;

/// The first built-in sample's resource ID.
pub const FIRST_ID: u16 = 1000;
/// The last built-in sample's resource ID.
pub const LAST_ID: u16 = 1045;
/// The number of built-in samples.
pub const COUNT: usize = (LAST_ID - FIRST_ID + 1) as usize;
/// The startup sound's ID. It is not a drum sample; the DLL loads it last, into slot 46.
pub const STARTUP_ID: u16 = 1024;

/// The TR-808 sample names in ID order, IDs 1000..=1023.
pub const NAMES_808: [&str; 24] = [
    "BD", "SD1", "SD2", "SD3", "SD4", "SD5", "N1", "N2", "N3", "N4", "CB", "CH", "CL", "CP", "CY",
    "HC", "HT", "LC", "LT", "MA", "MC", "MT", "OH", "RS",
];
/// The TR-909 sample names in ID order, IDs 1025..=1045.
pub const NAMES_909: [&str; 21] = [
    "bd1", "bd2", "bd3", "bd4", "sd1", "sd2", "sd3", "sd4", "sd5", "sdn1", "sdn2", "sdn3", "lt",
    "mt", "ht", "rs", "hc", "chh", "ohh", "ccy", "rcy",
];

/// The resource ID of TR-808 sample `i` (an index into [`NAMES_808`]).
pub const fn id_808(i: usize) -> u16 {
    FIRST_ID + i as u16
}

/// The resource ID of TR-909 sample `i` (an index into [`NAMES_909`]).
pub const fn id_909(i: usize) -> u16 {
    STARTUP_ID + 1 + i as u16
}

/// Why the samples could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleError {
    /// The file is not a PE image, or its resource directory is damaged.
    Pe(String),
    /// A resource is not a usable AIFF (the ID is in [`SampleError::Resource`]).
    Aiff(String),
    /// A built-in sample's ID is missing: not a ReBirth DLL.
    Missing(u16),
    /// An AIFF resource failed to parse.
    Resource(u16, Box<SampleError>),
    /// The file is a PE image with no `"AIFF"` resources.
    NoAiffResources,
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SampleError::Pe(m) => write!(f, "{m}"),
            SampleError::Aiff(m) => write!(f, "{m}"),
            SampleError::Missing(id) => write!(f, "the AIFF resource {id} is missing"),
            SampleError::Resource(id, e) => write!(f, "AIFF resource {id}: {e}"),
            SampleError::NoAiffResources => write!(f, "the file has no \"AIFF\" resources"),
        }
    }
}

impl std::error::Error for SampleError {}

/// One sample: `frames` of channel 0, then [`PADDING`] zero samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    data: Vec<i16>,
}

impl Sample {
    /// Parses an AIFF file (the bytes of one resource).
    pub fn from_aiff(bytes: &[u8]) -> Result<Sample, SampleError> {
        Ok(Sample {
            data: aiff::parse(bytes)?,
        })
    }

    /// A sample of the given audio, padded as a loaded one is.
    pub fn from_audio(audio: &[i16]) -> Sample {
        let mut data = Vec::with_capacity(audio.len() + PADDING);
        data.extend_from_slice(audio);
        data.resize(audio.len() + PADDING, 0);
        Sample { data }
    }

    /// The number of audio frames, not counting the padding.
    pub fn frames(&self) -> usize {
        self.data.len() - PADDING
    }

    /// The audio, `frames()` samples.
    pub fn audio(&self) -> &[i16] {
        &self.data[..self.frames()]
    }

    /// The audio followed by the 512 zero samples.
    pub fn padded(&self) -> &[i16] {
        &self.data
    }
}

/// The 46 built-in samples, by resource ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleSet {
    samples: Vec<Arc<Sample>>,
}

impl SampleSet {
    /// Loads the samples from the bytes of `ReBirth.dll`. The check that it is a ReBirth DLL is
    /// the resource set: every ID 1000..=1045 must be present and a valid AIFF (the same 46
    /// resources are in the 2.01 DLL; a DLL without them has nothing the engine could use).
    pub fn from_dll(dll: &[u8]) -> Result<SampleSet, SampleError> {
        let res = pe::named_type_resources(dll, "AIFF")?;
        if res.is_empty() {
            return Err(SampleError::NoAiffResources);
        }
        let mut samples = Vec::with_capacity(COUNT);
        for id in FIRST_ID..=LAST_ID {
            let r = res
                .iter()
                .find(|r| r.id == id)
                .ok_or(SampleError::Missing(id))?;
            samples.push(Arc::new(
                Sample::from_aiff(r.data).map_err(|e| SampleError::Resource(id, Box::new(e)))?,
            ));
        }
        Ok(SampleSet { samples })
    }

    /// The sample with resource ID `id` (1000..=1045).
    ///
    /// # Panics
    /// If `id` is outside the range.
    pub fn get(&self, id: u16) -> &Sample {
        &self.samples[(id - FIRST_ID) as usize]
    }

    /// The sample with resource ID `id`, shared (the drum voices keep the sample they play).
    ///
    /// # Panics
    /// If `id` is outside the range.
    pub fn shared(&self, id: u16) -> Arc<Sample> {
        self.samples[(id - FIRST_ID) as usize].clone()
    }

    /// A set made of `samples`, in resource ID order from 1000 (for tests and tools).
    ///
    /// # Panics
    /// If there are not exactly [`COUNT`] samples.
    pub fn from_samples(samples: Vec<Sample>) -> SampleSet {
        assert_eq!(samples.len(), COUNT);
        SampleSet {
            samples: samples.into_iter().map(Arc::new).collect(),
        }
    }

    /// A set made of shared `samples`, in resource ID order from 1000 (a mod's samples with the
    /// built-in ones in the slots it lacks).
    ///
    /// # Panics
    /// If there are not exactly [`COUNT`] samples.
    pub fn from_shared(samples: Vec<Arc<Sample>>) -> SampleSet {
        assert_eq!(samples.len(), COUNT);
        SampleSet { samples }
    }

    /// TR-808 sample `i` (an index into [`NAMES_808`]).
    pub fn tr808(&self, i: usize) -> &Sample {
        self.get(id_808(i))
    }

    /// TR-909 sample `i` (an index into [`NAMES_909`]).
    pub fn tr909(&self, i: usize) -> &Sample {
        self.get(id_909(i))
    }
}

/// What a mod change ([`crate::frame::Engine::set_samples`]) lets go of: the drum machines' old
/// sample tables and the samples their silenced voices still held. It has room for all of them
/// up front, so that the change frees nothing where it runs (the real-time player's audio
/// callback); its owner drops it elsewhere.
#[derive(Debug, Default)]
pub struct Retired {
    sets: [Option<Arc<SampleSet>>; Retired::SETS],
    samples: [Option<Arc<Sample>>; Retired::SAMPLES],
}

impl Retired {
    /// Room for the old tables: the TR-808's and the TR-909's.
    pub const SETS: usize = 2;
    /// Room for the voices' samples: 13 per drum machine (the 808's 5 SIMPLE, 3 FILTER,
    /// 3 PITCHED and the DUAL's two; the 909's 4, 2, 5 and two), with room to spare.
    pub const SAMPLES: usize = 32;

    /// Keeps `s`, or hands it back when full.
    pub fn keep_set(&mut self, s: Arc<SampleSet>) -> Result<(), Arc<SampleSet>> {
        match self.sets.iter_mut().find(|x| x.is_none()) {
            Some(slot) => {
                *slot = Some(s);
                Ok(())
            }
            None => Err(s),
        }
    }

    /// Keeps `s`, or hands it back when full.
    pub fn keep_sample(&mut self, s: Arc<Sample>) -> Result<(), Arc<Sample>> {
        match self.samples.iter_mut().find(|x| x.is_none()) {
            Some(slot) => {
                *slot = Some(s);
                Ok(())
            }
            None => Err(s),
        }
    }

    /// The tables kept.
    pub fn sets(&self) -> impl Iterator<Item = &Arc<SampleSet>> {
        self.sets.iter().flatten()
    }

    /// The samples kept.
    pub fn samples(&self) -> impl Iterator<Item = &Arc<Sample>> {
        self.samples.iter().flatten()
    }
}
