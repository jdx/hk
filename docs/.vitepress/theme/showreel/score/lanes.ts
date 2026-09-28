// Lanes: the seven steps run as bars on four file lanes. The full groove,
// Dm | C | Dm | G, with the tambourine, and over the G the concertina
// resolves motif M on D, which lifts into restore.
//
// Each lane has a pitch: README.md and src/app.ts sound together as D4 + F4
// (one prettier job), src/main.py is A4 and scripts/deploy.sh C5, so the
// four lanes at once are the Dm7 the reel lives in. The cues come from the
// kit's own schedule (kit/lanes.ts) and the scene's beat map
// (scenes/lanes-timing.ts BEATS), so they land where the chart moves. The
// big hit is go, all hands: four steps start at once, two writing their own
// files and two reading deploy.sh together, out of a breath of quiet, on a
// boot, the crew's hands, a low thump and a Dm stab, with every lane's note
// plucked together and all four padlocks clicking shut. After it every
// padlock clicks shut when its lock is taken, a little lighter for readers,
// and springs open when it is let go; a reader joining or leaving is a soft
// tick; and each single-lane step's ✔ rings its lane an octave up. The key
// that hands deploy.sh to shfmt's patch knocks as it lands. The two steps
// that read every file drop in together when the files are free, one run
// up the four lanes in double stops, a voice for each. The fiddle chops the
// off-beats.

import type { Part } from ".";
import { CASCADE, chipWidth, DOCK_CHIP, ganttAt, LANES, lanesOf, lockChanges, lockedFrom, SCHEDULE, type ScheduleStep } from "../kit/lanes";
import { BEATS } from "../scenes/lanes-timing";
import { type Section } from "../timeline";
import { C, CHOP, CHORD, chopBars, chordBars, DM, drumBars, G, HOME, melody, STOMP_FULL } from "./grooves";
import { ad, hz, line, type Mix, swell, sweep, X } from "./mix";
import { bassBar, blip, concertina, fiddlePluck, gangClap, jingle, knock, padlock, panX, ping, puff, shimmer, stamp, stomp, thump, tick, whoosh } from "./sounds";

/** Each lane's pitch: D4, F4, A4, C5. */
const LANE_NOTE = [62, 65, 69, 72];
/** The second voice of the two steps that read every file, a step higher: E4, G4, B4, D5. */
const HIGHER = [64, 67, 71, 74];
/** Go's stab, over the Dm: D5 F5 A5. */
const GO_CHORD = [74, 77, 81];
/** Where the padlocks sit. */
const LOCK = panX(LANES.lockX);
/** The steps that wait in the dock, and the centres of their chips there (the kit's dock slots). */
const DOCK_CHIPS = ganttAt(2).chips.filter((c) => c.dock);
const DOCKED = DOCK_CHIPS.map((c) => c.x + chipWidth(c.step.step, DOCK_CHIP) / 2);

/** A step that holds every file, dropping out of the dock. */
const holdsAll = (s: ScheduleStep): boolean => lanesOf(s).length === LANES.rows.length;
/** The steps that read every file: they drop together. */
const PASSES = SCHEDULE.filter(holdsAll);
/** The steps that start together on go. */
const CREW = SCHEDULE.filter((st) => st.start === BEATS.go);
/** The lanes they take, each once: two readers share deploy.sh. */
const CREW_LANES = [...new Set(CREW.flatMap(lanesOf))].sort((a, b) => a - b);

/**
 * Go, all hands: the air is cut off a 32nd short of it, so the hit lands
 * out of a breath, and four steps start at once: a boot on top of the
 * groove, the crew's hands, a low thump and a short Dm stab that carries it
 * on a small speaker while the music ducks, and every lane's note plucked
 * together, a few milliseconds apart so they sound as four.
 */
function allHands(m: Mix, s: Section): void {
  const t = s.beat(BEATS.go);
  const pan = panX(760);
  const suck = t - X / 2;
  whoosh(m, swell(t - 2 * X, t - X, 0.01, suck - 0.02, 0.05, suck), sweep(t - 2 * X, 1500, suck, 6000), 1.1, { send: 0.15 }, "white");
  m.duck(t, 0.6, 0.2);
  stomp(m, t, 1.2, pan * 0.5);
  gangClap(m, t, 0.95, "sfx");
  jingle(m, t, 1, 0.3, pan);
  thump(m, t, 0.55, 110, 42, 0.35, { pan });
  knock(m, t, hz(62), 0.4, pan, 0.14);
  concertina(m, t, t + 0.09, GO_CHORD, 0.17, { attack: 0.006, sustain: 0.55, release: 0.06, bright: 3800, pan, send: 0.2, bus: "sfx" });
  for (const lane of CREW_LANES) fiddlePluck(m, t + 0.003 * lane, hz(LANE_NOTE[lane]), 0.085, panX(648), 0.25, { bus: "sfx" });
}

/**
 * The steps that read every file drop out of the dock together once the
 * files are free: a light fall of air, a soft stamp as they land across the
 * four lanes, and a 32nd run up them in double stops, a voice for each.
 */
function passes(m: Mix, s: Section, steps: readonly ScheduleStep[]): void {
  const [first] = steps;
  const t = s.beat(first.start);
  const land = s.beat(lockedFrom(first));
  const pan = panX(first.x0);
  whoosh(m, ad(t, t + X / 4, 0.03, land - 0.005), sweep(t, 2600, land, 900), 1.4, { pan, send: 0.15, hold: false });
  stamp(m, land, 0.45, pan);
  LANE_NOTE.forEach((n, i) => {
    const at = land + (i * X) / 2 + 0.002;
    fiddlePluck(m, at, hz(n), 0.045, pan + 0.08 * (i - 1.5), 0.25, { bus: "sfx" });
    if (steps.length > 1) fiddlePluck(m, at + 0.004, hz(HIGHER[i]), 0.04, pan + 0.08 * (i - 1.5), 0.25, { bus: "sfx" });
  });
}

export const part: Part = {
  // The heart of the reel.
  level: 1.18,
  cues(m, s) {
    // The queue: the two dock chips slide in from the right as a train, a
    // 32nd apart, each with a small closed padlock that rattles as it settles.
    const q = BEATS.queue;
    whoosh(m, ad(s.beat(q), s.beat(q + 0.3), 0.06, s.beat(q + 0.55)), sweep(s.beat(q), 2800, s.beat(q + 0.5), 1200), 1.5, { pan: line(s.beat(q), 0.7, s.beat(q + 0.5), panX(1420)), send: 0.2 });
    DOCKED.forEach((x, k) => padlock(m, s.beat(q + k / 8 + 0.3), true, 0.16 - 0.03 * k, panX(x)));

    // b1, go: all hands.
    allHands(m, s);

    // The padlocks: shut as a writer or the first reader takes the file,
    // lighter for readers, open as the last lets go, and a soft tick as a
    // reader joins or leaves. Four at once are staggered a few
    // milliseconds, so they sound as four.
    LANES.rows.forEach((_, lane) => {
      const pan = LOCK + 0.03 * (lane - 1.5);
      for (const e of lockChanges(lane)) {
        const t = s.beat(e.at) + 0.004 * lane;
        if (e.change === "pips") tick(m, t, 3200, 0.12, pan, 0.06);
        else if (e.change === "shut") padlock(m, t, true, (e.at === BEATS.go ? 0.36 : 0.3) * (e.state === "read" ? 0.8 : 1), pan);
        else padlock(m, t, false, 0.2, pan);
      }
    });

    // A single-lane step's ✔ rings its lane an octave up.
    for (const st of SCHEDULE) {
      if (lanesOf(st).length > 1) continue;
      ping(m, s.beat(st.end) + 0.01, hz(LANE_NOTE[st.lanes[0]] + 12), 0.075, 0.45, { pan: panX(st.x1), send: 0.3 });
    }

    // A turn on deploy.sh: shellcheck lets it go and the key clicks as it
    // hops across, landing on shfmt's patch with a wooden knock on G4.
    const shfmtWrite = SCHEDULE.find((st) => st.step === "shfmt")!.phases[1];
    tick(m, s.beat(BEATS.shellcheckDone) + 0.01, 2200, 0.3, panX(LANES.lockX), 0.08);
    knock(m, s.beat(BEATS.shfmtWrite), hz(67), 0.3, panX(shfmtWrite.x0), 0.14);

    // depends: ruff's ✔ sends a spark along the bracket, which snaps taut
    // as it lands, and ruff-format starts on A4.
    const [ruff, ruffFormat] = ["ruff", "ruff-format"].map((n) => SCHEDULE.find((st) => st.step === n)!);
    whoosh(m, ad(s.beat(BEATS.dependsSpark), s.beat(BEATS.depends) - 0.02, 0.035, s.beat(BEATS.depends) + 0.01), sweep(s.beat(BEATS.dependsSpark), 2000, s.beat(BEATS.depends), 6000), 4, { pan: line(s.beat(BEATS.dependsSpark), panX(ruff.x1), s.beat(BEATS.depends), panX(ruffFormat.x0)), send: 0.2, hold: false }, "white");
    blip(m, s.beat(BEATS.depends), hz(81), 0.11);
    fiddlePluck(m, s.beat(BEATS.depends) + 0.004, hz(LANE_NOTE[2]), 0.07, panX(ruffFormat.x0), 0.22, { bus: "sfx", bright: 9 });

    // prettier ✔: the files it wrote flash, and the steps that read every
    // file drop in together.
    puff(m, s.beat(BEATS.clamp) + 0.01, 0.03, panX(SCHEDULE[0].x1), false);
    passes(m, s, PASSES);

    // b12: the ✔ cascade down the lanes, D5 F5 A5 D6, one a sixteenth.
    [74, 77, 81, 86].forEach((n, lane) => {
      ping(m, s.beat(CASCADE.beat + lane * CASCADE.each), hz(n), 0.1, 0.7, { pan: panX(CASCADE.x), send: 0.35 });
    });

    // A light crosses the finished chart.
    const [glint0, glint1] = BEATS.glint.map((b) => s.beat(b));
    shimmer(m, glint0, glint1, 0.018, 0);
    // The note and the dock clear away.
    const out = s.beat(BEATS.detailOut);
    whoosh(m, ad(out, out + 0.15, 0.045, out + 0.35), sweep(out, 1200, out + 0.35, 3000), 1.2, { pan: line(out, -0.2, out + 0.35, 0.4), send: 0.2 });
  },
  // The heart of the reel: the crew's hands at full strength. Go, on 2,
  // brings its own boot and hands, so the groove's hands hold back on it.
  drums: (m, s) => drumBars(m, s, [STOMP_FULL], 1, [], (t) => Math.abs(t - s.beat(BEATS.go)) < 1e-6),
  bass(m, s) {
    bassBar(m, s.bar(0), ...DM);
    bassBar(m, s.bar(1), ...C);
    bassBar(m, s.bar(2), ...DM);
    bassBar(m, s.bar(3), ...G);
  },
  lead: (m, s) => melody(m, s.beat(12), HOME),
  pads(m, s) {
    const chords = [CHORD.Dm, CHORD.C, CHORD.Dm, CHORD.G];
    chordBars(m, s, chords);
    // The fiddle chops the off-beats, harder than in the commit: the heart of the reel.
    chopBars(m, s, chords, 1.5 * CHOP, 0, 4);
  },
};
