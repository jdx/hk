// The groove over a whole section, in D dorian (D E F G A B C: a minor
// third with a major sixth, the B natural): the crew's stomp, the oom-pah
// bass, the concertina's chords and its tune, and the fiddle's chops, one
// entry per bar. The reel keeps 120 BPM throughout, so a passage that should
// breathe gets the half-time pattern (a boot on 1, hands on 3) rather than a
// slower tempo.

import { BEAT, type Section } from "../timeline";
import { hz, type Mix, X } from "./mix";
import {
  bassBar,
  type BassPattern,
  concertina,
  concertinaLine,
  fiddlePluck,
  gangClap,
  jingle,
  type Note,
  OOMPAH,
  type ReedOpts,
  stomp,
} from "./sounds";

/** One bar of drums in sixteenths: boots, the crew's hands, the tambourine, and its soft ghost notes. */
export type Drums = readonly [stomps: readonly number[], claps: readonly number[], jingles?: readonly number[], ghosts?: readonly number[]];

/** A boot on 1, hands on 3: for passages that breathe. */
export const HALF: Drums = [[0], [8]];
/** The stomp: boots on 1 and 3, the crew's hands on 2 and 4. */
export const STOMP: Drums = [[0, 8], [4, 12]];
/** The full stomp: a pickup boot on the and of 3, and the tambourine on the off-beat eighths with two ghosts. */
export const STOMP_FULL: Drums = [[0, 8, 10], [4, 12], [2, 6, 10, 14], [7, 15]];

/** A bass root as a MIDI note, and how much grit carries it to small speakers. */
export type Root = readonly [note: number, grit: number];
export const DM: Root = [38, 0.3];
export const C: Root = [36, 0.4];
export const AM: Root = [33, 0.75];
/** IV major: its B natural is the dorian colour. */
export const G: Root = [31, 0.9];

/** Chords for the concertina's middle register, under the tune and over the bass. */
export const CHORD = {
  Dm: [57, 62, 65],
  C: [55, 60, 64],
  /** C with the F for the E: under a tune whose F falls on the C bar's downbeat. */
  Csus4: [55, 60, 65],
  Am: [57, 60, 64],
  G: [55, 59, 62],
} as const;

/** Levels: the tune leads, the chords sit under it, and the fiddle's chops tick along. */
export const LEAD = 0.12;
export const BED = 0.07;
export const CHOP = 0.05;

/** The item for bar `b`: the list's own, or its last one once the list runs out. */
const nth = <T>(list: readonly T[], b: number): T => list[Math.min(b, list.length - 1)];

/** Reel times where the groove holds back, so a cue there is the only attack. */
export type Hush = (t: number) => boolean;
const never: Hush = () => false;

/**
 * One bar of drums from `t0`, less any hit `hush` holds back. The downbeat
 * boot lands hardest and a pickup lightest; the crew's hands and the
 * tambourine keep their places. The tambourine sits well under the boots:
 * it is nearly all top end, and it plays for a third of the reel.
 */
export function stompBar(m: Mix, t0: number, [stomps, claps, jingles = [], ghosts = []]: Drums, vel = 1, hands = 0.82, hush = never): void {
  const at = (s: number) => t0 + s * X;
  const heard = (s: number) => !hush(at(s));
  for (const s of stomps.filter(heard)) stomp(m, at(s), vel * (s === 0 ? 1 : s % 8 === 0 ? 0.9 : 0.7));
  // The crew's hands a little under the boots, so a cue on 2 or 4 still speaks.
  for (const s of claps.filter(heard)) gangClap(m, at(s), hands * vel);
  for (const s of jingles.filter(heard)) jingle(m, at(s), 0.5 * vel);
  for (const s of ghosts.filter(heard)) jingle(m, at(s), 0.2 * vel, 0.05);
}

/**
 * A bar of drums for each bar of the section; `hands` sets the crew's claps
 * against the boots. The groove's boots rest on `rests` (section beats),
 * where a cue brings its own boot, so the two never double; every hit rests
 * where `hush` says.
 */
export function drumBars(m: Mix, s: Section, bars: readonly Drums[], hands = 0.82, rests: readonly number[] = [], hush = never): void {
  for (let b = 0; b < s.bars; b++) {
    const [stomps, ...rest] = nth(bars, b);
    const free = stomps.filter((x) => !rests.some((at) => Math.abs(at - (4 * b + x / 4)) < 1e-6));
    stompBar(m, s.bar(b), [free, ...rest], 1, hands, hush);
  }
}

/** A bar of the staccato bass for each of the section's first `bars` bars. */
export function bassBars(m: Mix, s: Section, roots: readonly Root[], pattern: BassPattern = OOMPAH, bars = s.bars): void {
  for (let b = 0; b < bars; b++) bassBar(m, s.bar(b), ...nth(roots, b), pattern);
}

/**
 * The concertina's chords, one per bar, from its left hand (panned left of
 * the tune, which the right hand plays). The bellows turn on every bar line,
 * so even a chord that holds breathes with the bar.
 */
export function chordBars(m: Mix, s: Section, chords: readonly (readonly number[])[], level = BED, o: ReedOpts = {}): void {
  for (let b = 0; b < s.bars; b++) {
    concertina(m, s.bar(b), s.bar(b + 1), nth(chords, b), level, { attack: 0.05, sustain: 0.8, release: 0.08, bright: 2000, pan: -0.3, send: 0.22, ...o });
  }
}

/**
 * The fiddle chopping the off-beats, on the and of 2 and the and of 4 of
 * bars `from` to `to` (or on `beats` of each bar): the chord's top two
 * notes an octave up, plucked as a double stop, except where `hush` says.
 */
export function chopBars(
  m: Mix,
  s: Section,
  chords: readonly (readonly number[])[],
  vel = CHOP,
  from = 0,
  to = s.bars,
  beats: readonly number[] = [1.5, 3.5],
  hush = never,
): void {
  for (let b = from; b < to; b++) {
    const top = nth(chords, b).slice(-2);
    for (const beat of beats) {
      const t = s.bar(b) + beat * BEAT;
      if (!hush(t)) top.forEach((n, i) => fiddlePluck(m, t + 0.004 * i, hz(n + 12), vel, 0.35));
    }
  }
}

/**
 * The heartbeat: a muffled lub-dub on every other beat from beat `from` of
 * the section to before beat `to`, felt more than heard.
 */
export function heartbeat(m: Mix, s: Section, from = 0, to = s.bars * 4): void {
  for (let b = from; b < to; b += 2) {
    stomp(m, s.beat(b), 1, 0, true);
    stomp(m, s.beat(b) + X, 0.55, 0, true);
  }
}

/** A phrase of the tune: [start, length] in beats from its downbeat, MIDI note, and velocity. */
export type Phrase = readonly (readonly [beat: number, beats: number, midi: number, vel: number])[];

/**
 * Motif M, the hook: D5 (dotted quarter), D5, C5, A4 | F4 G4 A4 B4 in
 * eighths, C5, D5. Its B natural is the dorian colour.
 */
export const M: Phrase = [
  [0, 1.5, 74, 1],
  [1.5, 0.5, 74, 0.8],
  [2, 1, 72, 0.9],
  [3, 1, 69, 0.85],
  [4, 0.5, 65, 0.8],
  [4.5, 0.5, 67, 0.8],
  [5, 0.5, 69, 0.85],
  [5.5, 0.5, 71, 0.9],
  [6, 1, 72, 0.95],
  [7, 1, 74, 1],
];

/** M's answer, a step higher and home again: E5 (dotted quarter), E5, D5, C5 | A4 B4 C5 E5 in eighths, D5 held. */
export const ANSWER: Phrase = [
  [0, 1.5, 76, 1],
  [1.5, 0.5, 76, 0.8],
  [2, 1, 74, 0.9],
  [3, 1, 72, 0.85],
  [4, 0.5, 69, 0.8],
  [4.5, 0.5, 71, 0.8],
  [5, 0.5, 72, 0.85],
  [5.5, 0.5, 76, 0.9],
  [6, 2, 74, 1],
];

/** M's run resolved on D in one bar: F4 G4 A4 B4 C5 in eighths, then D5 held. */
export const HOME: Phrase = [
  [0, 0.5, 65, 0.8],
  [0.5, 0.5, 67, 0.8],
  [1, 0.5, 69, 0.85],
  [1.5, 0.5, 71, 0.9],
  [2, 0.5, 72, 0.9],
  [2.5, 1.5, 74, 1],
];

/**
 * A phrase on the concertina from `t0`. Runs of eighths are slurred under
 * one bellows; quarters and longer are lifted a little before the next note,
 * and a repeated note is struck again.
 */
export function melody(m: Mix, t0: number, phrase: Phrase, vel = LEAD, o: ReedOpts = {}): void {
  const notes = phrase.map(([b, len, n, v], i): Note => {
    const next = phrase[i + 1];
    let end = b + len - (len >= 1 ? 0.12 : 0);
    if (next && next[2] === n) end = Math.min(end, next[0] - 0.1);
    return [t0 + b * BEAT, t0 + end * BEAT, n, v];
  });
  // Darker than a bare reed, so the tune sings under the cues rather than
  // over them, and its reed harmonics (1.7 to 2.9 kHz) do not make it the
  // loudest voice on a small speaker.
  concertinaLine(m, notes, vel, { pan: 0.2, send: 0.2, bright: 2600, ...o });
}
