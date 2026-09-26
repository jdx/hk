// The score's sound palette: struck, plucked, and blown voices shared by
// every section, the drums and bass the groove is built from, and the pads,
// then the shanty's own voices: boots on the deck, the crew's claps, the
// tambourine, the concertina and the fiddle, the sea, and the ship.

import { BEAT } from "../timeline";
import { hash, inQuad, rng } from "../math";
import { ad, type Curve, glide, hold, hz, line, type Mix, type NoiseKind, perc, type Pt, swell, sweep, type Voice, type VoiceOpts, warmOf, X } from "./mix";

// Sound palette. Lowpass and highpass Q values are resonance in dB, as the
// Web Audio spec defines them; bandpass Q is the usual bandwidth ratio.

/** A small struck tone with a quick inharmonic shimmer on top. */
export function ping(m: Mix, t: number, f: number, vel: number, len: number, o: VoiceOpts = {}): void {
  const v = m.voice(perc(t, vel, 0.001, len), o);
  if (!v) return;
  v.osc("sine", f);
  v.osc("sine", f * 2.76, perc(t, 0.25, 0.0005, len * 0.3));
}

/** A tiny mechanical click with a pitched body. */
export function tick(m: Mix, t: number, f: number, vel: number, pan = 0, send = 0.06): void {
  const v = m.voice(perc(t, 0.7 * vel, 0.0004, 0.035), { pan, send });
  if (!v) return;
  v.noise("white", 1, v.filter("bandpass", f * 2.2, 2.2), 1, t + 0.05);
  v.osc("sine", f, perc(t, 0.86, 0.0004, 0.018));
}

/** A weighty low hit: a sine that drops in pitch, with a soft noise slap. */
export function thump(m: Mix, t: number, vel: number, f0: number, f1: number, len: number, o: VoiceOpts = {}): void {
  const v = m.voice(perc(t, 0.6 * vel, 0.0015, len), o);
  if (!v) return;
  v.osc("sine", [[t, f0], [t + 0.045, f1 * 1.25, "exp"], [t + len, f1, "exp"]]);
  v.noise("white", perc(t, 0.3, 0.0005, 0.035), v.filter("lowpass", 2400, -3), 1, t + 0.05);
}

/** A cork-like pop: a sine that chirps up to its pitch. */
export function pop(m: Mix, t: number, f: number, vel: number, pan = 0, send = 0.18): void {
  const v = m.voice(perc(t, vel, 0.0015, 0.17), { pan, send });
  if (!v) return;
  v.osc("sine", [[t, f * 0.42], [t + 0.02, f, "exp"], [t + 0.12, f * 1.015, "exp"]]);
  v.osc("triangle", sweep(t, f * 0.84, t + 0.02, f * 2), perc(t, 0.14, 0.0008, 0.05));
}

/** A glassy bell: struck-bar partials over a bright transient. */
export function ding(m: Mix, t: number, f: number, vel: number, pan: number, len = 1.3, send = 0.38): void {
  const v = m.voice(perc(t, vel, 0.0008, len), { pan, send });
  if (!v) return;
  const partials = [[1, 1, 1], [2, 0.3, 0.55], [2.76, 0.2, 0.35], [5.4, 0.1, 0.18], [8.93, 0.05, 0.1]];
  for (const [ratio, a, k] of partials) {
    if (f * ratio > 16000) continue;
    v.osc("sine", f * ratio, k === 1 ? a : perc(t, a, 0.0005, len * k));
  }
  v.noise("white", perc(t, 0.12, 0.0005, 0.06), v.filter("highpass", 7000, 0), 1, t + 0.08);
}

/** Glass on glass: inharmonic partials, short and bright. */
export function clink(m: Mix, t: number, vel: number, pan: number): void {
  const v = m.voice(perc(t, vel, 0.0004, 0.6), { pan, send: 0.25 });
  if (!v) return;
  const base = hz(98);
  for (const [ratio, a, len] of [[1, 1, 0.6], [1.59, 0.55, 0.35], [2.21, 0.4, 0.22], [2.93, 0.28, 0.14]]) {
    v.osc("sine", base * ratio, perc(t, a, 0.0004, len));
  }
  v.noise("white", perc(t, 0.5, 0.0003, 0.008), v.filter("highpass", 6000, 0), 1, t + 0.02);
}

/** Air: noise through a moving band. Sustained, so a mid-sweep start still hears it. */
export function whoosh(m: Mix, env: readonly Pt[], band: Curve, q: number, o: VoiceOpts = {}, kind: NoiseKind = "pink"): void {
  const v = m.voice(env, { hold: true, ...o });
  if (v) v.noise(kind, 1, v.filter("bandpass", band, q));
}

/**
 * A liquid bloop: a sine that chirps up into its pitch, with an octave ring
 * and overtones rung through a resonant filter that opens over them. The
 * chirp is short and the upper layers carry the surface, so a run of bloops
 * reads as liquid rather than a low-mid hum: a plink rising through 1 to 3
 * kHz as the bubble breaks keeps it wet on small speakers.
 */
export function bloop(m: Mix, t: number, f: number, vel: number, pan: number, len = 0.22): void {
  const v = m.voice(perc(t, 1.2 * vel, 0.005, len), { pan, send: 0.3 });
  if (!v) return;
  const rise = sweep(t, f * 0.7, t + 0.025, f);
  v.osc("sine", rise, 0.7);
  v.osc("sine", sweep(t, f * 1.4, t + 0.025, f * 2), perc(t, 0.45, 0.003, len * 0.45));
  // The formant opens as the bubble surfaces and closes as it rings out.
  const lp = v.filter("lowpass", [[t, f * 1.2], [t + 0.04, f * 8, "exp"], [t + len, f * 2, "exp"]], 10);
  v.osc("sawtooth", rise, 0.32, lp);
  const top = Math.min(3200, f * 4.2);
  v.osc("sine", sweep(t + 0.004, top * 0.55, t + 0.03, top), perc(t + 0.004, 0.5, 0.002, 0.06));
  v.osc("triangle", sweep(t + 0.004, top * 0.8, t + 0.03, top * 1.3), perc(t + 0.004, 0.14, 0.002, 0.035));
}

/** A cardboard knock with a pitched hollow body. */
export function knock(m: Mix, t: number, f: number, vel: number, pan: number, send = 0.12): void {
  const v = m.voice(perc(t, vel, 0.0008, 0.2), { pan, send });
  if (!v) return;
  v.noise("white", perc(t, 0.8, 0.0005, 0.06), v.filter("bandpass", 1300 + f, 1.3), 1, t + 0.08);
  v.osc("triangle", sweep(t, f * 2, t + 0.02, f), perc(t, 0.7, 0.001, 0.15));
  v.osc("sine", sweep(t, f, t + 0.05, f * 0.5), 0.45);
}

// Drums and bass for the groove.

export function kick(m: Mix, t: number, vel: number, filtered = false): void {
  const v = m.voice(perc(t, 0.5 * vel, 0.002, 0.46), { bus: filtered ? "music" : "drums" });
  if (!v) return;
  const into = filtered ? v.filter("lowpass", 160, -3) : v.amp;
  v.osc("sine", [[t, 165], [t + 0.032, 62, "exp"], [t + 0.3, 45, "exp"]], 1, into);
  if (!filtered) v.noise("white", perc(t, 0.22, 0.0004, 0.012), v.filter("highpass", 2500, 0), 1, t + 0.02);
}

export function clap(m: Mix, t: number, vel: number): void {
  // Three hands a few milliseconds apart, then the room.
  const env: Pt[] = [
    [t, 0],
    [t + 0.0008, 0.8 * vel],
    [t + 0.009, 0.15 * vel, "exp"],
    [t + 0.0098, 0.75 * vel],
    [t + 0.018, 0.15 * vel, "exp"],
    [t + 0.0188, 0.9 * vel],
    [t + 0.05, 0.28 * vel, "exp"],
    [t + 0.2, 0.0001, "exp"],
    [t + 0.204, 0],
  ];
  const v = m.voice(env, { bus: "drums", send: 0.2, pan: -0.05 });
  if (!v) return;
  v.noise("white", 1.6, v.filter("highpass", 700, 0, v.filter("bandpass", 1500, 1.1)));
  v.osc("sine", 1900, perc(t, 0.3, 0.0005, 0.012));
}

export function hat(m: Mix, t: number, vel: number): void {
  const v = m.voice(perc(t, 0.28 * vel, 0.0008, 0.05), { bus: "drums", pan: 0.22 });
  if (v) v.noise("white", 1, v.filter("highpass", 7200, 0));
}

/**
 * One bar of the groove from `t0`, in sixteenths: kicks and claps where
 * given, a hat on every off-beat eighth, and soft ghost hats where given.
 * The kicks also pump the bass and pads.
 */
export function grooveBar(
  m: Mix,
  t0: number,
  kicks: readonly number[],
  claps: readonly number[],
  ghosts: readonly number[] = [],
): void {
  for (const s of kicks) {
    m.kick(t0 + s * X);
    kick(m, t0 + s * X, s % 8 === 0 ? 1 : 0.7);
  }
  for (const s of claps) {
    clap(m, t0 + s * X, 1);
  }
  for (let s = 0; s < 16; s++) {
    if (s % 4 === 2) {
      hat(m, t0 + s * X, 1);
    } else if (ghosts.includes(s)) {
      hat(m, t0 + s * X, 0.4);
    }
  }
}

/**
 * A run of bass notes as one voice: the oscillators keep their phase from
 * note to note, so repeated notes never cancel, and the envelope dips at each
 * change to articulate it. Notes are [start, end, MIDI note, velocity].
 * `grit` is a band-passed saw at the root that puts the line's third to
 * sixth harmonics (150 to 300 Hz) about 11 dB under the fundamental, so the
 * line survives on laptop and phone speakers that roll off below 200 Hz.
 * With `gap`, a note that ends before the next begins decays like a plucked
 * string and stops dead at its end: the shanty's staccato oom-pah.
 */
export function bassRun(
  m: Mix,
  notes: readonly Note[],
  level = 0.08,
  cutoff = 360,
  grit = 0.3,
  gap = false,
): void {
  const env: Pt[] = [];
  const freq: Pt[] = [];
  let silent = true;
  notes.forEach(([a, b, n, vel], i) => {
    const peak = level * vel;
    freq.push(i === 0 ? [a, hz(n)] : [a, hz(n), "set"]);
    env.push([a, silent ? 0 : 0.3 * peak]);
    const next = notes[i + 1];
    if (gap && (!next || next[0] > b + 0.01)) {
      env.push([a + 0.008, peak], [Math.max(a + 0.01, b - 0.03), 0.6 * peak, "exp"], [b, 0.02 * peak, "exp"], [b + 0.004, 0]);
      silent = true;
    } else {
      env.push([a + 0.012, peak], [b - 0.02, 0.82 * peak]);
      silent = false;
    }
  });
  if (!silent) {
    const end = notes[notes.length - 1][1];
    env.push([end, 0.0001, "exp"], [end + 0.004, 0]);
  }
  const v = m.voice(env, { bus: "music", hold: true });
  if (!v) return;
  const lp = v.filter("lowpass", cutoff, -3);
  const sat = v.shaper(m.sh.sat, lp);
  v.osc("sine", freq, 0.7, sat);
  v.osc(
    "triangle",
    freq.map(([t, f, k]): Pt => [t, f * 2, k]),
    0.12,
    sat,
  );
  if (grit) v.osc("sawtooth", freq, grit, v.filter("bandpass", 210, 1.1, v.filter("lowpass", 420, -3)));
}

/** A note of a bass run or a tune: start and end in reel seconds, MIDI note, velocity. */
export type Note = readonly [start: number, end: number, midi: number, vel: number];

/** One bar of bass in sixteenths: [start, length, interval above the root, velocity]. */
export type BassPattern = readonly (readonly [start: number, len: number, interval: number, vel: number])[];

/**
 * The oom-pah: the root on 1 and 3, the fifth above it on 2 and 4, each
 * three sixteenths long, so every beat bounces off a sixteenth of silence.
 */
export const OOMPAH: BassPattern = [
  [0, 3, 0, 1],
  [4, 3, 7, 0.8],
  [8, 3, 0, 1],
  [12, 3, 7, 0.8],
];

/**
 * One bar of `pattern` from `t0` on MIDI note `root`, staccato. The lower
 * the root, the further its harmonics sit from the 210 Hz band that carries
 * them to small speakers, so low roots want more `grit`.
 */
export function bassBar(m: Mix, t0: number, root: number, grit: number, pattern: BassPattern = OOMPAH, level = 0.08): void {
  bassRun(
    m,
    pattern.map(([s, len, iv, vel]): Note => [t0 + s * X, t0 + (s + len) * X, root + iv, vel]),
    level,
    360,
    grit,
    true,
  );
}

/**
 * One pad chord: two detuned voices spread across the stereo field. `thin`
 * highpasses it and scoops 250 to 400 Hz, so a breakdown chord leaves the
 * low end to the downbeat and the low mids to the liquid.
 */
export function pad(
  m: Mix,
  t0: number,
  t1: number,
  notes: number[],
  level: number,
  cutoff: Curve,
  attack = 0.08,
  rel = 0.3,
  thin = 0,
): void {
  const each = level / notes.length;
  for (const side of [-1, 1]) {
    const v = m.voice(hold(t0, attack, each, t1, 0.9 * each, rel), { bus: "music", pan: side * 0.4, send: 0.3, hold: true });
    if (!v) continue;
    let into: AudioNode = v.amp;
    if (thin) {
      const scoop = v.filter("peaking", 320, 1, v.amp);
      scoop.gain.value = -5;
      into = v.filter("highpass", thin, -1, scoop);
    }
    const lp = v.filter("lowpass", cutoff, -1, into);
    for (const n of notes) v.osc(warmOf(m.ac, m.sh), hz(n), 1, lp, side * 7);
  }
}

// Swishes, springs, stamps, flutters and blips.

export function flick(m: Mix, t: number, pan: number, vel = 0.16): void {
  const v = m.voice(perc(t, 2.2 * vel, 0.002, 0.08), { pan, send: 0.12 });
  if (!v) return;
  v.noise("white", 1, v.filter("bandpass", sweep(t, 2200, t + 0.05, 7000), 2.5));
  v.osc("sine", sweep(t, 1400, t + 0.045, 3200), 0.35);
}

/** A jaw-harp spring: a resonant filter and the pitch wobbling together. */
export function boing(m: Mix, t: number, vel = 0.3, f = hz(50), len = 0.42): void {
  const v = m.voice(perc(t, vel, 0.004, len), { send: 0.15 });
  if (!v) return;
  const lp = v.filter("lowpass", 900, 9);
  const saw = v.osc("sawtooth", f, 0.5, lp);
  v.lfo("sine", 11, sweep(t, 700, t + len, 20), lp.frequency);
  v.lfo("sine", 11, sweep(t, f * 0.1, t + len, 0.3), saw.frequency);
  const s = v.osc("sine", sweep(t, f * 2.25, t + 0.05, f * 2), 0.35);
  v.lfo("sine", 11, sweep(t, f * 0.17, t + len, 0.5), s.frequency);
}

export function stamp(m: Mix, t: number, vel = 1, pan = 0.12): void {
  thump(m, t, 0.6 * vel, 190, 75, 0.22, { pan });
  const v = m.voice(perc(t, 0.4 * vel, 0.0006, 0.07), { pan, send: 0.15 });
  if (v) v.noise("white", 1, v.filter("bandpass", 1500, 0.9), 1, t + 0.1);
  const p = m.voice(perc(t + 0.004, 0.12 * vel, 0.0005, 0.03), { pan: pan + 0.08 });
  if (p) p.noise("white", 1, p.filter("highpass", 5500, 0), 1, t + 0.05);
}

/** Fabric spinning: a sharp first flap, then flaps gated fast, slowing as the spring settles. */
export function flutter(m: Mix, t: number, len = 0.2, vel = 0.3): void {
  const snap = m.voice(perc(t, 3 * vel, 0.0006, 0.03), { send: 0.12, pan: -0.2 });
  if (snap) {
    snap.noise("white", 1, snap.filter("bandpass", 2400, 0.9), 1, t + 0.04);
    snap.osc("sine", sweep(t, 900, t + 0.025, 380), 0.35);
  }
  const v = m.voice(hold(t + 0.004, 0.012, vel * 4, t + len * 0.5, vel * 3, len * 0.5), {
    send: 0.12,
    pan: line(t, -0.2, t + len, 0.2),
  });
  if (!v) return;
  const bp = v.filter("bandpass", sweep(t, 1600, t + len, 900), 1.4);
  const am = v.vca(0.5, bp);
  v.lfo("square", sweep(t, 34, t + len, 12), 0.45, am.gain);
  v.noise("pink", 1, am);
}

export function blip(m: Mix, t: number, f: number, vel: number): void {
  const v = m.voice(perc(t, 1.6 * vel, 0.001, 0.035), { send: 0.1 });
  if (v) v.osc("sine", sweep(t, f * 1.3, t + 0.02, f));
}

// The shanty: the crew's boots on the deck and their claps, the tambourine,
// the concertina and the fiddle, the sea, the ship's timbers and bell, and
// the fishing reel. Nothing is sampled. Each event varies a little, seeded
// by its own time (`vary`), so the crew sounds human while every start
// point still hears the same sounds.

/** A per-event variation in [-1, 1]: the same for the same event from any start point. */
function vary(t: number, seed: number): number {
  return hash(Math.round(t * 9973), seed) * 2 - 1;
}

/**
 * A boot on a wooden deck: a low thud, the deck's plank modes (where a
 * laptop speaker hears the boot), the heel's knock on the wood and the scuff
 * of the sole, and no high click, so it reads as boots and not a drum
 * machine. It pumps the music like a kick. `muffled` puts it below decks
 * (lowpassed, on the music bus, no pump): the heartbeat.
 */
export function stomp(m: Mix, t: number, vel = 1, pan = 0, muffled = false): void {
  const k = 1 + 0.04 * vary(t, 11);
  if (muffled) {
    const v = m.voice(perc(t, 0.22 * vel, 0.003, 0.32), { bus: "music", pan });
    if (!v) return;
    const lp = v.filter("lowpass", 190, -3);
    v.osc("sine", [[t, 110 * k], [t + 0.05, 58, "exp"], [t + 0.3, 46, "exp"]], 1, lp);
    v.osc("triangle", sweep(t, 150 * k, t + 0.06, 96 * k), perc(t, 0.5, 0.002, 0.12), lp);
    return;
  }
  m.kick(t);
  thump(m, t, 0.6 * vel, 120 * k, 48, 0.26, { bus: "drums", pan });
  // The planks: four short modes, each dropping a little as the board settles.
  const b = m.voice(perc(t, 0.3 * vel, 0.001, 0.12), { bus: "drums", pan, send: 0.12 });
  if (b) {
    for (const [f, a, len] of [[176, 0.6, 0.1], [243, 1, 0.08], [331, 0.6, 0.06], [468, 0.35, 0.04]] as const) {
      b.osc("sine", sweep(t, f * k * 1.06, t + 0.03, f * k), perc(t, a, 0.001, len));
    }
  }
  const s = m.voice(perc(t, 0.3 * vel, 0.001, 0.07), { bus: "drums", pan, send: 0.12 });
  if (s) {
    s.noise("pink", 1.3, s.filter("bandpass", 380 * k, 1.2), 1, t + 0.09);
    s.noise("white", perc(t, 0.8, 0.0006, 0.03), s.filter("bandpass", 850 * k, 1.4), 1, t + 0.04);
  }
}

/**
 * One pair of hands: a sharp burst and a smaller one as the palms meet
 * again, rung through the hollow between them (`band`: cupped hands are
 * lower, flat ones brighter).
 */
export function handClap(m: Mix, t: number, vel: number, pan = 0, band = 1500, send = 0.2): void {
  const env: Pt[] = [
    [t, 0],
    [t + 0.0008, vel],
    [t + 0.006, 0.22 * vel, "exp"],
    [t + 0.0068, 0.55 * vel],
    [t + 0.02, 0.22 * vel, "exp"],
    [t + 0.13, 0.0001, "exp"],
    [t + 0.134, 0],
  ];
  const v = m.voice(env, { bus: "drums", send, pan });
  if (v) v.noise("white", 1.6, v.filter("highpass", 600, 0, v.filter("bandpass", band, 1.3)));
}

/**
 * The crew on 2 and 4: three pairs of hands a few milliseconds apart and
 * spread across the deck, each its own shape. A crew, not one clapper. The
 * first hand is on the beat; the others are only ever late.
 */
export function gangClap(m: Mix, t: number, vel = 1): void {
  const hands = [
    [0, -0.3, 1, 1250],
    [0.006, 0, 0.8, 1750],
    [0.013, 0.25, 0.7, 2250],
  ] as const;
  hands.forEach(([dt, pan, v, band], i) => {
    const late = i ? 0.001 * (1 + vary(t, 20 + i)) : 0;
    handClap(m, t + dt + late, vel * v * (1 + 0.1 * vary(t, 30 + i)), pan, band * (1 + 0.06 * vary(t, 40 + i)));
  });
}

/** A tambourine's metal: three narrow ringing bands and a highpassed hiss, summed into `into`. */
function zils(v: Voice, into: AudioNode): AudioNode {
  const split = v.m.ac.createGain();
  for (const [f, q, a] of [[5300, 9, 2.2], [7700, 10, 1.8], [10200, 8, 1.3]] as const) {
    split.connect(v.gain(a, v.filter("bandpass", f, q, into)));
  }
  split.connect(v.gain(0.3, v.filter("highpass", 6500, 0, into)));
  return split;
}

/** A tambourine struck: its jingles ringing, gated at about 30 Hz so they rattle instead of hiss. */
export function jingle(m: Mix, t: number, vel: number, len = 0.12, pan = 0.25): void {
  const v = m.voice(perc(t, 0.42 * vel, 0.001, len), { bus: "drums", pan, send: 0.08 });
  if (!v) return;
  const am = v.vca(0.62);
  v.lfo("square", 29 + 4 * vary(t, 50), 0.38, am.gain);
  v.noise("white", 1, zils(v, am));
}

/** A tambourine shaken from `t0` to `t1`: the jingles rattling at the hand's rate, swelling and dying away. */
export function jingleRoll(m: Mix, t0: number, t1: number, vel: number, pan = 0.25): void {
  const v = m.voice(hold(t0, 0.12, 0.16 * vel, t1 - 0.1, 0.1 * vel, 0.18), { bus: "drums", pan, send: 0.12, hold: true });
  if (!v) return;
  const shake = v.vca(0.55);
  v.lfo("sine", 11, 0.35, shake.gain);
  const rattle = v.vca(0.7, shake);
  v.lfo("square", 31, 0.25, rattle.gain);
  v.noise("white", 1, zils(v, rattle));
}

const reeds = new WeakMap<BaseAudioContext, PeriodicWave>();

/**
 * A free reed's wave: odd and even harmonics falling slower than a saw's,
 * with a strong second and third, so it buzzes like a squeezebox instead of
 * glowing like a pad. The concertina's lowpass sets where it stops.
 */
export function reedOf(ac: BaseAudioContext): PeriodicWave {
  let w = reeds.get(ac);
  if (!w) {
    const H = 48;
    const real = new Float32Array(H);
    const imag = new Float32Array(H);
    for (let k = 1; k < H; k++) imag[k] = k ** -0.8 * (k === 2 ? 1.35 : k === 3 ? 1.2 : 1);
    w = ac.createPeriodicWave(real, imag);
    reeds.set(ac, w);
  }
  return w;
}

export interface ReedOpts {
  pan?: number;
  send?: number;
  /** Seconds the reeds take to speak. */
  attack?: number;
  /** Seconds the sound takes to stop after `t1`. */
  release?: number;
  /** Where the bellows pressure has fallen to by `t1`, as a fraction of the peak. */
  sustain?: number;
  /** The lowpass ceiling: lower for a chord that sits under the tune. */
  bright?: number;
  /** How fast each note's two reeds beat against each other, in Hz. */
  beat?: number;
  /** Depth of a 6 Hz bellows shake (tremolo); none by default. */
  shake?: number;
}

/**
 * A concertina sounding `notes` (MIDI) from `t0` to `t1`. Each note is two
 * reeds tuned a few hertz apart, so they beat at the same rate up and down
 * the keyboard as a tuned box does; a formant near 1.1 kHz gives the box its
 * nasal colour, the reeds' upper harmonics bloom as they start to speak,
 * and a breath of air escapes the valve as the button opens. Chords are
 * scaled so they sit near a single note's level.
 */
export function concertina(m: Mix, t0: number, t1: number, notes: readonly number[], vel: number, o: ReedOpts = {}): void {
  const attack = o.attack ?? 0.03;
  const env = hold(t0, attack, vel, t1, (o.sustain ?? 0.85) * vel, o.release ?? 0.06);
  const v = m.voice(env, { bus: "music", pan: o.pan ?? 0, send: o.send ?? 0.18, hold: true });
  if (!v) return;
  let into: AudioNode = v.amp;
  if (o.shake) {
    const am = v.vca(1 - o.shake);
    v.lfo("sine", 6, o.shake, am.gain);
    into = am;
  }
  const top = o.bright ?? 4500;
  const lp = v.filter("lowpass", [[t0, top * 0.5], [t0 + attack + 0.05, top, "exp"]], 0, into);
  const formant = v.filter("peaking", 1100, 1.1, lp);
  formant.gain.value = 4;
  const wave = reedOf(m.ac);
  const beat = o.beat ?? 2.6;
  const each = 1 / Math.sqrt(notes.length);
  for (const n of notes) {
    const f = hz(n);
    const cents = (1200 * Math.log2(1 + beat / f)) / 2;
    v.osc(wave, f, 0.56 * each, formant, -cents);
    v.osc(wave, f, 0.44 * each, formant, cents);
  }
  v.noise("pink", perc(t0, 0.05, 0.004, 0.06), v.filter("bandpass", 2400, 0.9, lp), 1, t0 + 0.08);
}

/**
 * A tune on the concertina: [start, end, MIDI note, velocity] per note, one
 * voice each. A note that runs into a different next note overlaps it by 10
 * ms, legato under the same bellows; a gap is left as written.
 */
export function concertinaLine(m: Mix, notes: readonly Note[], vel: number, o: ReedOpts = {}): void {
  notes.forEach(([a, b, n, v], i) => {
    const next = notes[i + 1];
    const slur = next && next[2] !== n && next[0] - b < 0.02;
    concertina(m, a, slur ? next[0] + 0.01 : b, [n], vel * v, { attack: 0.02, release: 0.04, sustain: 0.9, ...o });
  });
}

/**
 * A pizzicato fiddle: the string plucked a touch sharp and settling to
 * pitch, its brightness closing fast, through the body's air and wood
 * resonances, with the finger's snap on top. Higher strings ring shorter.
 */
export function fiddlePluck(m: Mix, t: number, f: number, vel: number, pan = 0.2, send = 0.2): void {
  const len = Math.min(0.5, 0.18 + 60 / f);
  const v = m.voice(perc(t, vel, 0.002, len), { bus: "music", pan, send });
  if (!v) return;
  const air = v.filter("peaking", 280, 1.6);
  air.gain.value = 3;
  const body = v.filter("peaking", 450, 1.4, air);
  body.gain.value = 4;
  const lp = v.filter("lowpass", [[t, Math.min(14000, 7 * f)], [t + 0.15, 1.6 * f, "exp"]], 4, body);
  const pitch: Pt[] = [[t, f * 1.008], [t + 0.03, f, "exp"]];
  v.osc("sawtooth", pitch, 0.55, lp);
  v.osc("triangle", pitch, 0.45, lp);
  v.noise("white", perc(t, 0.35, 0.0005, 0.01), v.filter("bandpass", 2500, 1.2), 1, t + 0.02);
}

/**
 * A bowed slide up the fiddle from `f0` to `f1`, arriving at `t1` and held
 * `sustain` seconds: a sawtooth, as a bowed string moves, gliding faster as
 * it arrives, vibrato growing on the held note, the rosin's hiss, and the
 * body's resonances.
 */
export function fiddleSlide(m: Mix, t0: number, t1: number, f0: number, f1: number, vel: number, sustain = 0.25, pan = 0.2): void {
  const end = t1 + sustain;
  const v = m.voice(hold(t0, 0.05, vel, end, 0.8 * vel, 0.12), { bus: "music", pan, send: 0.25, hold: true });
  if (!v) return;
  const air = v.filter("peaking", 280, 1.4);
  air.gain.value = 3;
  const body = v.filter("peaking", 480, 1.2, air);
  body.gain.value = 3;
  const bridge = v.filter("peaking", 2900, 1.3, body);
  bridge.gain.value = 4;
  const lp = v.filter("lowpass", 6500, 0, bridge);
  const saw = v.osc("sawtooth", [...glide(t0, t1, f0, f1, inQuad), [end + 0.12, f1]], 1, lp);
  v.lfo("sine", 5.6, [[t0, 0], [t1, 0.002 * f1], [end, 0.012 * f1]], saw.frequency);
  v.noise("white", 0.06, v.filter("bandpass", 3200, 0.8, lp));
}

/**
 * The sea: a wave rolls in from `t0` (pink noise through a band that rises
 * as it builds, over the water's low body), breaks just before halfway, and
 * drains away in a fizz of foam until `t1`, crossing the stereo field.
 */
export function wave(m: Mix, t0: number, t1: number, vel: number, pan = 0): void {
  const crest = t0 + 0.45 * (t1 - t0);
  const across = line(t0, pan - 0.35, t1, pan + 0.35);
  const band: Pt[] = [[t0, 300], [crest, 1200, "exp"], [t1, 400, "exp"]];
  whoosh(m, swell(t0, t0 + 0.35 * (crest - t0), 0.25 * vel, crest, vel, t1), band, 0.8, { send: 0.35, pan: across });
  const r = m.voice(swell(t0, t0 + 0.5 * (crest - t0), 0.3 * vel, crest + 0.1, 0.8 * vel, t1), { send: 0.2, hold: true, pan });
  if (r) r.noise("pink", 1, r.filter("lowpass", 380, 0));
  const f = m.voice(ad(crest - 0.15, crest + 0.1, 0.3 * vel, t1), { send: 0.4, hold: true, pan: across });
  if (f) {
    const hp = f.filter("highpass", sweep(crest, 2800, t1, 1600), 0);
    f.noise("white", 0.5, hp);
    f.noise("crackle", 1.5, hp);
  }
}

/**
 * A ship's timber under strain from `t0` to `t1`: the wood catching and
 * letting go (a slow pulse train, quickening as the strain builds and easing
 * after) rung through two low wooden formants.
 */
export function creak(m: Mix, t0: number, t1: number, vel = 1, pan = 0.1): void {
  const r = rng(Math.round(t0 * 997) + 707);
  const rate: Pt[] = [];
  for (let t = t0; t <= t1 + 1e-9; t += 0.015) rate.push([t, 16 + 14 * Math.sin(((t - t0) / (t1 - t0)) * Math.PI) + 5 * r()]);
  const v = m.voice(hold(t0, 0.08, 0.7 * vel, t1 - 0.05, 0.35 * vel, 0.07), { send: 0.16, hold: true, pan });
  if (!v) return;
  const saw = v.osc("sawtooth", rate, 1, v.filter("bandpass", sweep(t0, 480, t1, 400), 7));
  saw.connect(v.gain(0.6, v.filter("bandpass", sweep(t0, 1150, t1, 980), 9)));
}

/** Something tearing over `len` seconds: crackle and hiss through a closing band, gated by a slowing sawtooth. */
export function crackle(m: Mix, t: number, len = 0.12, vel = 0.2, pan = 0): void {
  const e = t + len;
  const env: Pt[] = [[t, 0], [t + 0.01, vel], [Math.max(t + 0.02, e - 0.03), 0.7 * vel], [e, 0.0001, "exp"], [e + 0.004, 0]];
  const v = m.voice(env, { pan, send: 0.12 });
  if (!v) return;
  const am = v.vca(0.6, v.filter("bandpass", sweep(t, 3000, e, 1500), 0.9));
  v.lfo("sawtooth", sweep(t, 120, e, 40), 0.4, am.gain);
  v.noise("crackle", 2.5, am, sweep(t, 2.4, e, 0.8));
  v.noise("white", 0.5, am);
}

/** A blade drawn: a scrape through a band sweeping up, over three inharmonic rings. */
export function shing(m: Mix, t: number, vel = 0.2, pan = 0): void {
  const v = m.voice(perc(t, vel, 0.002, 0.32), { pan, send: 0.3 });
  if (!v) return;
  v.noise("white", perc(t, 4, 0.001, 0.14), v.filter("bandpass", sweep(t, 3800, t + 0.12, 7600), 7), 1, t + 0.2);
  for (const [f, a, len] of [[3150, 0.35, 0.3], [4730, 0.28, 0.22], [6920, 0.2, 0.14]] as const) {
    v.osc("sine", f, perc(t, a, 0.001, len));
  }
}

/** A glint from `t0` to `t1`: tiny bells running up D dorian from D7, over a breath of air. */
export function shimmer(m: Mix, t0: number, t1: number, vel = 0.05, pan = 0.1): void {
  const notes = [98, 100, 101, 103, 105, 107, 108, 110];
  notes.forEach((n, i) => {
    const t = t0 + ((t1 - t0) * i) / notes.length + 0.005 * (1 + vary(t0, 60 + i));
    ping(m, t, hz(n), vel * (1 - 0.06 * i), 0.18, { pan: pan + 0.3 * vary(t0, 70 + i), send: 0.4 });
  });
  const a = m.voice(ad(t0, (t0 + t1) / 2, 0.8 * vel, t1 + 0.2), { send: 0.4, pan, hold: true });
  if (a) a.noise("white", 1, a.filter("highpass", sweep(t0, 6000, t1, 9000), 0));
}

/**
 * A fishing reel's ratchet: the pawl clicking over the gear from `t0` to
 * `t1`, the gap between clicks easing from `d0` to `d1` (the line running
 * out faster or slower) and each click's pitch from `f0` to `f1`.
 */
export function ratchet(m: Mix, t0: number, t1: number, d0: number, d1: number, vel: number, f0 = 3400, f1 = f0, pan = 0): void {
  const span = t1 - t0;
  for (let t = t0, i = 0; t < t1 - 1e-6 && i < 512; i++) {
    const u = (t - t0) / span;
    const f = f0 * (f1 / f0) ** u;
    const v = m.voice(perc(t, vel * (0.85 + 0.15 * hash(i, 90)), 0.0003, 0.014), { pan, send: 0.08 });
    if (v) {
      v.noise("white", 3, v.filter("bandpass", f, 6), 1, t + 0.02);
      v.osc("sine", f * 0.31, perc(t, 0.6, 0.0003, 0.01));
    }
    t += d0 * (d1 / d0) ** u;
  }
}

export interface RiserOpts {
  vel?: number;
  /** Times at which the roll's gate doubles its rate: eighths, then sixteenths, then 32nds... */
  doubles?: readonly number[];
  /** Rising fifths under the noise; true by default. */
  fifths?: boolean;
  /** The gated noise roll; true by default. */
  roll?: boolean;
}

/**
 * A build that cuts dead at `t1`: noise rising through a band, fifths
 * rising an octave (A over E, into the D of the downbeat after), and a noise
 * roll whose gate opens every eighth and doubles its rate at each of
 * `doubles`. Each rate runs whole cycles from `t0`, so a start in the middle
 * picks the gate up on its grid. With `fifths` and `roll` off it is a plain
 * filtered-noise riser.
 */
export function riser(m: Mix, t0: number, t1: number, o: RiserOpts = {}): void {
  const vel = o.vel ?? 1;
  const cut = (a: number, peak: number): Pt[] => [
    [t0, 0],
    [t0 + Math.min(0.25, (t1 - t0) / 3), a * vel],
    [t1 - 0.006, peak * vel, "exp"],
    [t1, 0.0001, "exp"],
    [t1 + 0.003, 0],
  ];
  const n = m.voice(cut(0.012, 0.2), { send: 0.05, hold: true });
  if (n) n.noise("white", 1, n.filter("bandpass", sweep(t0, 400, t1, 7000), 1.1));
  if (o.fifths !== false) {
    const f = m.voice(cut(0.02, 0.16), { send: 0.06, hold: true });
    if (f) {
      const lp = f.filter("lowpass", sweep(t0, 400, t1, 5000), 3);
      for (const det of [-10, 10]) f.osc("sawtooth", sweep(t0, hz(45), t1, hz(57)), 0.5, lp, det);
      f.osc("sawtooth", sweep(t0, hz(52), t1, hz(64)), 0.35, lp);
    }
  }
  if (o.roll !== false) {
    const doubles = o.doubles ?? [];
    const r = m.voice(cut(0.01, 0.2), { send: 0.04, hold: true, pan: 0.05 });
    if (r) {
      const am = r.vca(0.5, r.filter("bandpass", sweep(t0, 900, t1, 3600), 0.9));
      const period = (t: number) => (2 * X) / 2 ** doubles.filter((d) => d <= t + 1e-9).length;
      const cycle = (t: number) => t0 + Math.ceil((t - t0) / period(t) - 1e-9) * period(t);
      const rate: Pt[] = [[t0, 2 / BEAT], ...doubles.map((d, i): Pt => [d, 2 ** (i + 2) / BEAT, "set"])];
      r.lfo("square", rate, 0.5, am.gain, cycle);
      r.noise("white", 1, am);
    }
  }
}

/**
 * A ship's bell: bronze, not glass. A hum an octave down and a minor-third
 * tierce give it a bell's colour, and the tierce and nominal are each a
 * slightly mistuned pair, so the ring warbles as it fades. `f` is the strike
 * note; the partials sum to about three times `vel`.
 */
export function bell(m: Mix, t: number, f: number, vel: number, pan = 0, len = 2.4, send = 0.4): void {
  const v = m.voice(perc(t, vel, 0.001, len), { pan, send });
  if (!v) return;
  // [ratio to the strike note, level, decay as a fraction of len, beat in Hz]
  const partials = [
    [0.5, 0.35, 1, 0],
    [1, 0.7, 0.8, 0],
    [1.2, 0.45, 0.65, 1.3],
    [1.5, 0.18, 0.45, 0],
    [2, 0.55, 0.5, 2.1],
    [2.5, 0.2, 0.3, 0],
    [3, 0.14, 0.22, 0],
    [4.2, 0.08, 0.14, 0],
  ] as const;
  for (const [ratio, a, k, beat] of partials) {
    const p = f * ratio;
    if (p > 16000) continue;
    v.osc("sine", p, k === 1 ? a : perc(t, a, 0.0006, len * k));
    if (beat) v.osc("sine", p + beat, perc(t, 0.6 * a, 0.0006, len * k));
  }
  v.noise("white", perc(t, 0.2, 0.0004, 0.02), v.filter("bandpass", 3600, 0.8), 1, t + 0.04);
}
