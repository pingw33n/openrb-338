# Examples

Songs written for this repository, as material to study and to change. They are not part of the
reference corpus (`tests/songs/`, `tests/refs/`): nothing here is compared against the original, and
nothing here is a test input.

## empuje

A dance piece of 64 bars at 118 BPM, built to test the techniques in `docs/beats.md`:

- **the drums carry the tune**: the 808's three toms are tuned far apart and play a phrase, while
  the 909 keeps time underneath;
- **micro-rhythm**: the figure is varied by one step across copies of the pattern, and the rim runs
  on a different clock from the toms, so the two never line up twice;
- **space**: about one sample in eight is silence, and the bass plays one or two notes a bar;
- **the drop-out**: the beat leaves twice and the layer that stays holds the room;
- **slap bass** from the corpus settings (`cutoff` 84, `resonance` 84, `accent` 118, a short
  `decay`, a little distortion), carried to the end by adding notes rather than volume;
- **minor**, on the four positions the shipped songs actually use (0, 7, 4, 3 — root, fifth, third,
  minor third).

`empuje.py` generates the spec and `empuje.rbs` is the song it produced. To hear it:

```sh
python3 examples/empuje.py /tmp/empuje.json      # the spec, from the generator
python3 tools/rbs.py build /tmp/empuje.json /tmp/empuje.rbs
./target/release/orb render /tmp/empuje.rbs -o /tmp/empuje.wav
xdg-open /tmp/empuje.wav
```

Or through the MCP server, which is the point of `orb_build` and `orb_song` (`docs/mcp.md`): read
`empuje.rbs`, change a value, write it back, render it, without a shell.

It is a study, not a finished record: it has no mix beyond what the machine's own levels give, and
the master sits where `docs/beats.md` says the mix stays legible. Change it.
