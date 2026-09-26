// Commit: `git commit` slams in and hk's hook starts. The stomp carries on
// over Dm | C | Dm, and the fiddle chops the and of 2 and the and of 4.
//
// The picture's cues: `git` drops from above with a boot and a falling
// thump, and its dust settles; `commit` smears in from the right and slams
// against it on the crew's hands and a knock, and the letters of `git`
// jostle; the message flips in a word an eighth, each a pluck up the Dm7
// (D5 F5 A5 C6) with a paper flap; Enter is a key's click and a thump while
// the drums duck and the pane opens; hk's header arrives on a soft ping,
// and the staged files' ✔ on a pluck and a quiet bell. The beats are the
// scene's own (scenes/commit.ts COMMIT_BEATS).

import type { Part } from ".";
import { COMMIT_BEATS as B } from "../scenes/commit";
import { bassBars, C, CHORD, chopBars, chordBars, DM, drumBars, STOMP } from "./grooves";
import { ad, hz, line, sweep } from "./mix";
import { ding, fiddlePluck, flick, gangClap, knock, panX, ping, puff, stomp, thump, tick, whoosh } from "./sounds";

const CHORDS = [CHORD.Dm, CHORD.C, CHORD.Dm];

/** `git commit` spans x 510 to 1410: `git` ends at 780, `commit` starts at 870. */
const GIT = panX(645);
const COMMIT = panX(1140);

/** The message's words flip in on the eighths (B.flips), left to right, up the Dm7. */
const FLIPS = [
  { note: 74, x: 620 },
  { note: 77, x: 830 },
  { note: 81, x: 1060 },
  { note: 84, x: 1300 },
] as const;

export const part: Part = {
  cues(m, s) {
    // b1: `git` drops from above, squashes, and throws up its dust.
    m.duck(s.beat(B.git), 0.25, 0.15);
    whoosh(m, ad(s.beat(B.git) - 0.16, s.beat(B.git) - 0.01, 0.035, s.beat(B.git) + 0.02), sweep(s.beat(B.git) - 0.16, 3200, s.beat(B.git), 900), 1.6, { pan: GIT, send: 0.12, hold: false });
    stomp(m, s.beat(B.git), 0.9, GIT);
    thump(m, s.beat(B.git), 0.5, 120, 48, 0.3, { pan: GIT });
    puff(m, s.beat(B.git) + 0.01, 0.05, GIT);

    // b1.5: `commit` smears in from the right edge and slams against `git`.
    whoosh(m, ad(s.beat(B.slam) - 0.22, s.beat(B.slam) - 0.015, 0.05, s.beat(B.slam) + 0.03), sweep(s.beat(B.slam) - 0.22, 1200, s.beat(B.slam), 4200), 1.4, { pan: line(s.beat(B.slam) - 0.22, 0.75, s.beat(B.slam), COMMIT), send: 0.15, hold: false });
    m.duck(s.beat(B.slam), 0.2, 0.12);
    gangClap(m, s.beat(B.slam), 0.9);
    knock(m, s.beat(B.slam), hz(62), 0.3, panX(870), 0.14);
    // The letters of `git` jostle: three small knocks, each softer, A4 C5 D5.
    [0.05, 0.1, 0.16].forEach((dt, i) => knock(m, s.beat(B.slam) + dt, hz(69 + [0, 3, 5][i]), 0.07 / (i + 1), GIT + 0.1 * (i - 1), 0.1));

    // b2 to b3.5: the message flips in a word an eighth, up the Dm7.
    FLIPS.forEach((f, i) => {
      const t = s.beat(B.flips[i]);
      flick(m, t, panX(f.x), 0.1);
      fiddlePluck(m, t + 0.004, hz(f.note), 0.09, panX(f.x), 0.22, { bus: "sfx", bright: 9 });
    });

    // b4, Enter: the key's click and a thump; the drums duck; the type
    // squashes into row 0 as the pane grows around it.
    m.duck(s.beat(B.enter), 0.55, 0.3);
    tick(m, s.beat(B.enter), 1300, 0.55, 0, 0.1);
    thump(m, s.beat(B.enter), 0.5, 300, 140, 0.16);
    whoosh(m, ad(s.beat(B.enter), s.beat(B.enter + 0.25), 0.08, s.beat(B.run + 0.1)), sweep(s.beat(B.enter), 900, s.beat(B.run), 2600), 1.1, { send: 0.25 });

    // b4.5: hk's pre-commit header and the spinner.
    ping(m, s.beat(B.run), hz(74), 0.07, 0.6, { pan: -0.2, send: 0.3 });
    // b5.5: ✔ files: the staged files are fetched.
    fiddlePluck(m, s.beat(B.files), hz(81), 0.1, -0.3, 0.25, { bus: "sfx", bright: 9 });
    ding(m, s.beat(B.files) + 0.004, hz(81), 0.045, -0.3, 0.9, 0.35);
  },
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, C, DM]),
  pads(m, s) {
    chordBars(m, s, CHORDS);
    chopBars(m, s, CHORDS);
  },
};
