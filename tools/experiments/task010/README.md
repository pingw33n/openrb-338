# task-010: determinism of the original's export per capture protocol

The scripts and songs behind `docs/export_state.md` section 11. The WAVs are not in git: re-run the
captures to get them (about 25 s of machine time per fresh-process export). Python is
`/usr/bin/python3`; paths below are relative to the repository root, `OUT` is any local directory.

## Songs

- The five songs: `tests/songs/diag/songs/{silence,303_accent0,808_snare,delay_straight_fb64,comp_ch_a100_t10}.rbs`.
- `zz_other_drums_kit.rbs` (protocol P4) is a byte copy of `tests/songs/diag/songs/drums_kit.rbs`
  (SHA-256 `5461741463bdbf0e19b21366fd522cbf5f8e84c4f5efcf5f9317f60edf7daff3`); the name only keeps its
  export out of the analysis.
- `songs/*_no303.{json,rbs}`: `808_snare` and `delay_straight_fb64` with both 303s silenced, built by
  `tools/experiments/task010/mkvar.py tools/experiments/task010/songs 808_snare delay_straight_fb64`.

## Captures (each line is one `tools/rb_export.py` run; it takes all the permits itself)

`X` = `tools/rb_export.py --restore-prf`, `S` = the five songs above, `R` = 1, 2, 3.

| Run directory | Command |
|---|---|
| `OUT/p1_rR` | `X --startup-sound off -o OUT/p1_rR S` |
| `OUT/p2_rR` | `X --startup-sound off --exports 2 -o OUT/p2_rR S` |
| `OUT/p3_rR` | `X --startup-sound off --play-seconds 3 -o OUT/p3_rR S` |
| `OUT/p4_rR_<song>` (one run per song) | `X --startup-sound off --reuse-process -o OUT/p4_rR_<song> zz_other_drums_kit.rbs <song>.rbs` |
| `OUT/pT3_r1`, `OUT/pT30_r1` | `X --startup-sound off --wait-after-open 3` (or `30`) `-o ... S` |
| `OUT/psndon0_r1`, `OUT/psndon10_r1` | `X --startup-sound on --wait-after-open 0` (or `10`) `-o ... silence.rbs 808_snare.rbs` |
| `OUT/pv_rR` | `X --startup-sound off -o OUT/pv_rR songs/808_snare_no303.rbs songs/delay_straight_fb64_no303.rbs` |
| `OUT/p5x_r1` | `X --startup-sound off --exports 5 -o OUT/p5x_r1 808_snare.rbs` |

That is 32 runs (9 for P1-P3, 15 for P4, 2 + 2 extras, 3 for pv, 1 for p5x).

## Analysis

- `analyze.py OUT`: SHA-256 of every export and `tools/rbcmp.py` for every pair inside a protocol.
- `xcmp.py OUT p1_r1/808_snare.wav:p3_r1/808_snare.wav ...`: pairs across protocols.
- `shift.py A.wav B.wav 1000 3000 400`: rms difference at no shift and at the best time shift (the
  303 phase test), e.g. `OUT/p1_r1/303_accent0.wav OUT/p2_r1/303_accent0.1.wav`.
- `phase.py OUT/p1_r1/303_accent0.wav OUT/*/303_accent0*.wav`: the phase offset of each export
  against the reference, fitted to a whole number `k` of idle 1024-frame buffers (the `k` fit that
  rbhost validation needs).
- `snare.py chain OUT/p5x_r1/808_snare.{1,2,3,4,5}.wav`: the 808 snare counter at the start of each
  of five exports in one process (result: 0, 9, 18, 27, 4).
- `snare.py ref OUT/p1_r1/808_snare.wav OUT/p3_r1/808_snare.wav OUT/p4_r1_808_snare/808_snare.wav`:
  the counter after a 3 s play (7) and after a `drums_kit` export (4).
