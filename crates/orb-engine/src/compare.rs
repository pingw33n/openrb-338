//! Comparing a render with its reference: the first differing sample, located in time and in
//! bars, for the corpus golden test (`tests/corpus_golden.rs`) and `orb compare`.
//!
//! The pass criterion is a byte-exact WAV (task-013): the whole file, header included. Both
//! sides here are interleaved int16 stereo, so the header can only differ in the frame count,
//! which [`Difference::Length`] reports.

use crate::frame::export::ExportRange;
use crate::song::TICKS_PER_BAR;

/// The output sample rate.
pub const RATE: u32 = 44100;

/// The first place where a render and its reference differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difference {
    /// The first differing sample: its frame, its channel (0 left, 1 right) and both values.
    Sample {
        frame: usize,
        channel: usize,
        got: i16,
        want: i16,
    },
    /// Every common sample is equal, but the frame counts differ.
    Length { got: usize, want: usize },
}

/// The first difference of `got` and `want` (interleaved stereo), or `None` when they are equal.
pub fn first_difference(got: &[i16], want: &[i16]) -> Option<Difference> {
    if let Some(i) = got.iter().zip(want).position(|(a, b)| a != b) {
        return Some(Difference::Sample {
            frame: i / 2,
            channel: i % 2,
            got: got[i],
            want: want[i],
        });
    }
    (got.len() != want.len()).then_some(Difference::Length {
        got: got.len() / 2,
        want: want.len() / 2,
    })
}

/// The time of `frame` from the start of the export, in seconds.
pub fn seconds(frame: usize) -> f64 {
    frame as f64 / RATE as f64
}

/// The song position of `frame` of an export of `range`, in bars counted from 0 as in the file
/// (a bar is four beats, 768 ticks): the loop start plus the frame's time at the song's tempo.
/// The tempo is the song's global one (BPM * 1000; ReBirth has no tempo automation), and the
/// shuffle, which moves the steps but not the bars, is ignored.
pub fn bar(range: ExportRange, frame: usize) -> f64 {
    let beats = seconds(frame) * (range.tempo as f64 / 1000.0) / 60.0;
    range.loop_start as f64 / TICKS_PER_BAR as f64 + beats / 4.0
}

/// One line describing `d` for an export of `range`: the frame, its time and bar, and for a
/// sample difference the channel and both values (`orb` the render, `ref` the reference).
pub fn describe(d: Difference, range: ExportRange) -> String {
    match d {
        Difference::Sample {
            frame,
            channel,
            got,
            want,
        } => format!(
            "first difference at frame {frame} ({:.6} s, bar {:.3}), channel {} ({}): orb {got}, \
             ref {want}",
            seconds(frame),
            bar(range, frame),
            channel,
            if channel == 0 { "left" } else { "right" },
        ),
        Difference::Length { got, want } => format!(
            "equal up to frame {} ({:.6} s, bar {:.3}), but orb has {got} frames and ref {want}",
            got.min(want),
            seconds(got.min(want)),
            bar(range, got.min(want)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_streams_have_no_difference() {
        assert_eq!(first_difference(&[1, 2, 3, 4], &[1, 2, 3, 4]), None);
        assert_eq!(first_difference(&[], &[]), None);
    }

    #[test]
    fn the_first_differing_sample_is_located() {
        let d = first_difference(&[0, 0, 5, 7, 9, 9], &[0, 0, 5, 8, 0, 0]);
        assert_eq!(
            d,
            Some(Difference::Sample {
                frame: 1,
                channel: 1,
                got: 7,
                want: 8
            })
        );
    }

    #[test]
    fn a_length_difference_after_equal_samples() {
        assert_eq!(
            first_difference(&[1, 2], &[1, 2, 3, 4]),
            Some(Difference::Length { got: 1, want: 2 })
        );
    }

    #[test]
    fn bars_follow_the_tempo_and_the_loop_start() {
        // 120 BPM: a bar is 2 s, 88200 frames
        let r = ExportRange {
            tempo: 120_000,
            loop_start: 3 * TICKS_PER_BAR,
            loop_end: 5 * TICKS_PER_BAR,
        };
        assert_eq!(bar(r, 0), 3.0);
        assert_eq!(bar(r, 88200), 4.0);
        assert_eq!(bar(r, 44100), 3.5);
        assert_eq!(seconds(44100), 1.0);
    }

    #[test]
    fn descriptions_name_the_frame_time_bar_channel_and_values() {
        let r = ExportRange {
            tempo: 120_000,
            loop_start: 0,
            loop_end: TICKS_PER_BAR,
        };
        let s = describe(
            Difference::Sample {
                frame: 44100,
                channel: 0,
                got: -3,
                want: 4,
            },
            r,
        );
        assert_eq!(
            s,
            "first difference at frame 44100 (1.000000 s, bar 0.500), channel 0 (left): orb -3, \
             ref 4"
        );
        let s = describe(Difference::Length { got: 10, want: 12 }, r);
        assert!(s.contains("orb has 10 frames and ref 12"), "{s}");
    }
}
