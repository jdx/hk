// Catch: a later commit is caught on the hook. The groove drops to half
// time over a heartbeat and a D2 pedal, under a dark Dm; the ✗ on b3 cuts
// the drums and the chord. The pedal holds through the snag and the yank,
// and the stomp returns on b6 with the bass bouncing again.
//
// The picture's cues: the sea washes right to left as the main line pans;
// hk's blocked run redraws in the inset, its step ✔ rows landing on plucks
// (D5 F5 A5) and its other rows on soft ticks, while the commit card glides
// in along the line on a creaking timber and brakes on b3. The ✗ is a low
// thump, a clank on the rail, and a concertina cluster, E3 against F3 with
// E4 against F4 over it. The line whips down, the
// hook snags the card's loop on a spring and the crew's hands, two boots
// heave and ho, and the yank lifts it off the rail on a rising breath. The
// timber creaks at each end of its swing, and the reel's ratchet winds it up
// out of frame, quickening from 32nds to 64ths.

import type { Part } from ".";
import { blocked } from "../kit/screens";
import { smoothstep } from "../math";
import { SNAG, STOP, SWING } from "../scenes/catch";
import { BEAT, type Section } from "../timeline";
import { BED, CHORD, DM, HALF, heartbeat, STOMP, stompBar } from "./grooves";
import { ad, hz, line, sweep, X } from "./mix";
import { bassBar, bassRun, boing, concertina, creak, fiddlePluck, gangClap, knock, pad, panX, ping, ratchet, stab, stamp, stomp, thump, tick, wave, whoosh } from "./sounds";

/** The card stops at x 1240 and hangs there; the empty slot is at x 1560. */
const CARD = panX(1240);
const SLOT = panX(1560);
/** The inset is x 120 to 880. */
const INSET = panX(500);

/**
 * Beats of the scene (scenes/catch.ts) it keeps to itself: the card rides
 * in from b1 and brakes from b2.2 to its stop; the line drops over b3.5 to
 * b3.875; two tugs; the yank from b5.25; the hook's glint at b6.25; the
 * reel winds up over b10 to b11.5.
 */
const RIDE = 1;
const BRAKE = 2.2;
const DROP = [3.5, 3.875] as const;
const TUGS = [4.5, 5] as const;
const YANK = 5.25;
const GLINT = 6.25;
const REEL = [10, 11.5] as const;

/** When the inset shows each of blocked's frames 0 to 13 (storyboard 6.7). */
const FRAME_BEATS = [1, 1.25, 1.5, 1.75, 2, 2.125, 2.25, 2.375, 2.5, 2.625, 2.75, 3, 3.125, 3.25];

/** The step ✔ rows ring D5, F5, A5 in the order they land. */
const STEP_NOTES = [74, 77, 81];

/** The rows each frame adds: [beat, row text]. */
function newRows(): [number, string][] {
  const out: [number, string][] = [];
  blocked.forEach((rows, f) => {
    const before = new Set<string>(f ? blocked[f - 1] : []);
    for (const r of rows.slice(1)) if (!before.has(r) && /^[✔✗] /.test(r)) out.push([FRAME_BEATS[f], r]);
  });
  return out;
}

/**
 * The card's swing, degrees, section-local seconds (scenes/catch.ts
 * swingAt): θ = A·e^(−d/2 beats)·sin(2πd/2 beats) from SWING, eased in over
 * the yank, with A held down to about 3.8° for the first lobe (A = 7°·(1 −
 * 0.6·e^(−d/0.75 beats))) and 7° from the second apex on.
 */
function swingAt(lt: number): number {
  const ramp = smoothstep(5.55 * BEAT, 6.1 * BEAT, lt);
  if (ramp <= 0) return 0;
  const d = lt - SWING;
  const amp = 7 * (1 - 0.6 * Math.exp(-Math.max(0, d) / (0.75 * BEAT)));
  return amp * Math.exp(-d / (2 * BEAT)) * Math.sin((2 * Math.PI * d) / (2 * BEAT)) * ramp;
}

/** The swing's apexes before the reel takes the card up, found on a 1 ms grid: [time, |angle| in degrees]. */
function apexes(s: Section): [number, number][] {
  const out: [number, number][] = [];
  const dt = 0.001;
  for (let lt = SWING; lt + dt < REEL[0] * BEAT; lt += dt) {
    const [a, b, c] = [swingAt(lt - dt), swingAt(lt), swingAt(lt + dt)];
    if (Math.abs(b) > 0.2 && Math.abs(b) >= Math.abs(a) && Math.abs(b) > Math.abs(c)) out.push([s.start + lt, Math.abs(b)]);
  }
  return out;
}

export const part: Part = {
  cues(m, s) {
    // b0 to b1: the main line pans left under a wash of sea.
    wave(m, s.start, s.beat(2.4), 0.07, 0.05, -1);

    // b1 to b3.25: hk's blocked run redraws in the inset.
    let step = 0;
    for (const [b, row] of newRows()) {
      const t = s.beat(b);
      if (row.startsWith("✗")) continue;
      if (/^✔ (files|stash)/.test(row)) tick(m, t, 1900, 0.3, INSET, 0.08);
      else fiddlePluck(m, t, hz(STEP_NOTES[Math.min(step++, STEP_NOTES.length - 1)]), 0.09, INSET, 0.25, { bus: "sfx", bright: 9 });
    }

    // The commit card rides in along the line on a creaking timber, and
    // brakes harder into its stop. The creak lets go just before it, so the
    // ✗ lands out of a breath.
    const brake = s.beat(BRAKE);
    const stop = s.at(STOP);
    creak(m, s.beat(RIDE), brake + 0.1, 0.3, line(s.beat(RIDE), -0.7, brake, panX(1100)));
    creak(m, brake, stop - 0.03, 0.4, line(brake, panX(1100), stop - 0.03, CARD));

    // The ✗, the poster's moment: a low thump, a clank of iron on the rail
    // (a knock on E4 and a stamp, which a small speaker hears), and a
    // cluster on the concertina's reeds, E against F, low and an octave up.
    m.duck(stop, 0.5, 0.3);
    thump(m, stop, 0.6, 95, 38, 0.55, { pan: CARD });
    knock(m, stop, hz(64), 0.35, CARD, 0.16);
    stamp(m, stop, 0.6, CARD);
    const cluster = { attack: 0.01, sustain: 0.35, release: 0.25, bright: 2500, pan: CARD, send: 0.3, bus: "sfx" } as const;
    concertina(m, stop, s.at(SNAG), [52, 53], 0.1, cluster);
    concertina(m, stop, s.at(SNAG), [64, 65], 0.07, cluster);

    // The line whips down from the top edge, paying out.
    const [drop0, drop1] = DROP.map((b) => s.beat(b));
    whoosh(m, ad(drop0, drop0 + 0.11, 0.05, drop1 + 0.06), sweep(drop0, 4200, drop1 + 0.06, 600), 1.8, { pan: CARD, send: 0.2 });
    ratchet(m, drop0, drop1, 0.02, 0.04, 0.07, 3400, 2600, CARD);

    // The snag: the bowl hooks the loop and the line goes taut.
    const snag = s.at(SNAG);
    m.duck(snag, 0.3, 0.2);
    boing(m, snag, 0.26, hz(50), 0.42);
    gangClap(m, snag, 1);
    // Two tugs, "heave" and "ho", the line straining at each.
    for (const b of TUGS) {
      stomp(m, s.beat(b), 1, CARD * 0.5);
      creak(m, s.beat(b), s.beat(b) + 0.16, 0.3, CARD);
    }
    // The yank lifts the card off the rail.
    const yank = s.beat(YANK);
    const swing = s.at(SWING);
    whoosh(m, ad(yank, swing - 0.05, 0.09, swing + 0.05), sweep(yank, 500, swing, 2800), 1.3, { pan: CARD, send: 0.25 });
    creak(m, yank, swing, 0.24, CARD);

    // A small ✗ flashes over the empty slot: the cluster again, quietly. No commit.
    stab(m, swing, [52, 53], 0.05, SLOT);
    // The hook glints.
    ping(m, s.beat(GLINT), hz(93), 0.025, 0.4, { pan: CARD, send: 0.45 });

    // The card swings on the line, the timber creaking at each end.
    apexes(s).forEach(([t, deg], i) => creak(m, t - 0.06, t + 0.16, 0.9 * (deg / 7), CARD + (i % 2 ? -0.15 : 0.15)));

    // The reel winds the card up out of frame, quickening from 32nds to 64ths.
    ratchet(m, s.beat(REEL[0]), s.beat(REEL[1]), X / 2, X / 4, 0.14, 3000, 4400, CARD);
  },
  drums(m, s) {
    stompBar(m, s.bar(0), HALF);
    heartbeat(m, s, 0, 3);
    stompBar(m, s.bar(1), [[8], [12]]);
    stompBar(m, s.bar(2), STOMP);
  },
  bass(m, s) {
    // The pedal's grit is the A minor root's, so a phone still hears the D2.
    bassRun(m, [[s.start, s.beat(6), 38, 1]], 0.04, 260, 0.8);
    bassBar(m, s.bar(1), ...DM, [
      [8, 3, 0, 1],
      [12, 3, 7, 0.8],
    ]);
    bassBar(m, s.bar(2), ...DM);
  },
  pads(m, s) {
    pad(m, s.start, s.beat(3) - 0.05, [50, 57, 62, 65], 0.07, 1400, 0.3, 0.05, 150);
    const o = { attack: 0.05, sustain: 0.8, release: 0.08, bright: 2000, pan: -0.3, send: 0.22 };
    concertina(m, s.beat(6), s.bar(2), CHORD.Dm, BED, o);
    concertina(m, s.bar(2), s.end, CHORD.Dm, BED, o);
  },
};
