// The reel's soundtrack, synthesized with the Web Audio API: oscillators,
// seeded noise, filters, and envelopes, with no samples and no network. The
// whole score is written in reel seconds and scheduled from any start point,
// so playback, seeking, and offline export hear the same mix.
//
// Sound design leads: every choreographed accent in the scenes has its own
// sound, tuned to D dorian where it has a pitch. Music supports it: a sea
// shanty at 120 BPM (boots and the crew's claps, a staccato oom-pah bass,
// a concertina's chords and its tune) that ducks under the effects, a
// breakdown under the morph that drops to a heartbeat and rises into a
// sixteenth of silence, and the resolve on the end card.
//
// This file is the master chain and the scheduling. The score itself is in
// score/: one module per section (score/index.ts), the sound palette they
// share (score/sounds.ts), and the voices and curves under them
// (score/mix.ts).

import type { ReelFacts } from "./facts";
import { compose } from "./score";
import { type Dip, floats, type Floats, MARGIN, Mix, noiseOf, roomOf, shared } from "./score/mix";
import { GAP } from "./score/morph";
import { DURATION, sec } from "./timeline";

export interface ScoreHandle {
  /** Fade out over ~30 ms and release every node. */
  stop(): void;
}

/** How far ahead of a live context's clock the duck curves start, which cannot start late without shifting whole. */
const CURVE_MARGIN = 0.06;
/** Seconds of score a live context gets before playScore returns; the rest follows on a timer. */
const FIRST = 0.4;
/** How far ahead of the clock the timer keeps the score built, and how often it runs. */
const AHEAD = 1.2;
const TICK_MS = 250;

/** Duck curves are sampled on a fixed grid from reel time 0. */
const DIP_STEP = 0.0025;

/**
 * A gain curve over the whole reel that is the product of dips: each accent
 * pulls the gain down over 8 ms and lets it recover exponentially (to under
 * 1% of its depth after five time constants).
 */
function dips(lists: readonly (readonly [readonly Dip[], number])[]): Floats {
  const n = Math.ceil((DURATION + 0.1) / DIP_STEP) + 1;
  const vals = floats(n).fill(1);
  for (const [list, scale] of lists) {
    for (const [te, d, r] of list) {
      const i0 = Math.max(0, Math.ceil((te - 0.008) / DIP_STEP));
      const i1 = Math.min(n - 1, Math.floor((te + r * 5) / DIP_STEP));
      const k = Math.exp(-DIP_STEP / r);
      let s = 0;
      for (let i = i0; i <= i1; i++) {
        const dt = i * DIP_STEP - te;
        if (dt < 0) vals[i] *= 1 - d * scale * ((dt + 0.008) / 0.008);
        else {
          // The recovery advances by a fixed ratio per step.
          s = s ? s * k : Math.exp(-dt / r);
          vals[i] *= 1 - d * scale * s;
        }
      }
    }
  }
  return vals;
}

/**
 * Play a whole-reel dip curve on `p` from reel time `t0`. Chromium moves a
 * curve whose start has already passed up to the clock, which would put
 * every dip after it late. The audio thread can run a whole device buffer
 * ahead of the clock the page reads, so a live score joins the curve at
 * least CURVE_MARGIN ahead of it, easing in from the resting gain over 10
 * ms; only dips in the first moments of a cold start are lost.
 */
function dipCurve(m: Mix, p: AudioParam, t0: number, vals: Floats): void {
  let k = Math.ceil(t0 / DIP_STEP - 1e-9);
  const late = m.live ? Math.ceil((m.clock(CURVE_MARGIN) - 1e-9) / DIP_STEP) : 0;
  const skip = late > k;
  k = Math.min(vals.length - 2, Math.max(k, late));
  const rest = vals.slice(k);
  if (skip) for (let i = 0; i < Math.min(4, rest.length); i++) rest[i] = 1 + (rest[i] - 1) * (i / 4);
  p.setValueCurveAtTime(rest, m.at(k * DIP_STEP), (rest.length - 1) * DIP_STEP);
}

/**
 * What a score's first pass records never changes between scores with the
 * same facts (only which voices get built does), so it is worked out once
 * per page, by the first score: the two duck curves. The facts can move the
 * benchmark race's cues, and their ducks with them.
 */
interface Plan {
  drums: Floats;
  music: Floats;
}
let planCache: { facts: ReelFacts | null; plan: Plan } | null = null;

function planOf(m: Mix): Plan {
  // A gentler pump than a club kick's: the music breathes with the boots,
  // and a tune note on the downbeat still speaks.
  const pump = m.kicks.map((t): Dip => [t, 0.25, 0.1]);
  return {
    // The groove ducks under the effects; the bass and the concertina also pump with the boots.
    drums: dips([[m.ducks, 0.6]]),
    music: dips([
      [m.ducks, 1],
      [pump, 1],
    ]),
  };
}

/**
 * Schedule the soundtrack into `dest`, starting `from` seconds into the reel
 * at audio-context time `when`. An offline context gets the whole score at
 * once; a live one gets its first moments before this returns and the rest
 * from a timer that stays a little over a second ahead of the clock. Pass
 * the same `facts` the picture draws, so the race's cues land on its bars.
 */
export function playScore(ac: BaseAudioContext, dest: AudioNode, from: number, when: number, facts: ReelFacts | null = null): ScoreHandle {
  if (!(from < DURATION - 0.03)) return { stop() {} };
  const sh = shared(ac);
  const live = !("startRendering" in ac);
  // Nearly every voice uses white noise; making it now, before any voice
  // checks the clock, keeps that cost out of the first voice that needs it.
  noiseOf(ac, sh, "white");

  // Master: a DC blocker, a touch less sub and more air, a glue compressor,
  // makeup, a soft ceiling, the end fade, and the stop() fade.
  const pre = ac.createGain();
  // The staccato bass and the boots' falling thuds are gated low tones, each
  // leaving a little DC and subsonic rumble; a microphone's coupling would
  // take it out, so this does, below anything a speaker plays.
  const dc = ac.createBiquadFilter();
  dc.type = "highpass";
  dc.frequency.value = 18;
  dc.Q.value = -3;
  const low = ac.createBiquadFilter();
  low.type = "lowshelf";
  low.frequency.value = 90;
  low.gain.value = -1.5;
  const air = ac.createBiquadFilter();
  air.type = "highshelf";
  air.frequency.value = 7000;
  air.gain.value = 2.5;
  const comp = ac.createDynamicsCompressor();
  comp.threshold.value = -18;
  comp.knee.value = 10;
  comp.ratio.value = 2;
  comp.attack.value = 0.005;
  const makeup = ac.createGain();
  // Half, because the ceiling curve's domain is ±2.
  makeup.gain.value = 0.5 * 1.9;
  const ceiling = ac.createWaveShaper();
  ceiling.curve = sh.ceiling;
  const tail = ac.createGain();
  const kill = ac.createGain();
  pre.connect(dc).connect(low).connect(air).connect(comp).connect(makeup).connect(ceiling).connect(tail).connect(kill).connect(dest);

  const sfx = ac.createGain();
  sfx.connect(pre);
  const drums = ac.createGain();
  drums.gain.value = 0.62;
  const drumDuck = ac.createGain();
  drums.connect(drumDuck).connect(pre);
  const music = ac.createGain();
  music.gain.value = 0.9;
  const musicDuck = ac.createGain();
  music.connect(musicDuck).connect(pre);

  const verb = ac.createGain();
  const conv = ac.createConvolver();
  // Building the impulse costs ~10 ms of main thread; a live score does it
  // from its first timer tick, which still lands well before `when`.
  if (!live) conv.buffer = roomOf(ac, sh);
  const verbLow = ac.createBiquadFilter();
  verbLow.type = "highpass";
  verbLow.frequency.value = 220;
  const verbOut = ac.createGain();
  verbOut.gain.value = 2.5;
  const verbGate = ac.createGain();
  verb.connect(conv).connect(verbLow).connect(verbOut).connect(verbGate).connect(pre);

  const m = new Mix(ac, sh, from, when, { sfx, drums, music }, verb, live);
  let stopped = false;
  let built = -Infinity;
  const build = (to: number): void => {
    m.lo = built;
    m.hi = to >= DURATION ? Infinity : to;
    m.floor = m.clock(live ? MARGIN : 0);
    compose(m, facts);
    m.collect = false;
    built = to;
  };
  // The first sounds go out first. The first score on a page also records
  // the whole score's ducks and kicks in this pass; later ones reuse them.
  const cached = planCache?.facts === facts ? planCache.plan : null;
  m.collect = !cached;
  build(live ? from + FIRST : DURATION);
  const p = cached ?? (planCache = { facts, plan: planOf(m) }).plan;
  const start = m.clock(live ? MARGIN : 0);

  // A fresh compressor starts clamped down and takes ~200 ms to open; a fast
  // release until just after the first sound lets it settle at once.
  comp.release.value = 0.001;
  comp.release.setValueAtTime(0.2, m.at(start) + 0.03);
  dipCurve(m, drumDuck.gain, start, p.drums);
  dipCurve(m, musicDuck.gain, start, p.music);
  // The breath before the resolve: the room goes quiet with everything else.
  const resolve = sec("end").start;
  m.set(verbGate.gain, [[GAP - 0.012, 1], [GAP + 0.01, 0.02, "exp"], [resolve - 0.004, 0.02], [resolve, 1]]);
  // Everything, reverb included, is silent before the last frame.
  const end = DURATION;
  m.set(tail.gain, [[end - 0.4, 1], [end - 0.14, 0.25, "exp"], [end - 0.035, 0.0002, "exp"], [end - 0.028, 0]]);

  let timer: ReturnType<typeof setTimeout> | undefined;
  const topUp = (): void => {
    timer = undefined;
    if (stopped) return;
    const clock = from + (ac.currentTime - when);
    const to = Math.min(DURATION, Math.max(clock, from) + AHEAD);
    if (to > built) build(to);
    if (built < DURATION) timer = setTimeout(topUp, TICK_MS);
  };
  if (live) {
    timer = setTimeout(() => {
      timer = undefined;
      if (stopped) return;
      conv.buffer = roomOf(ac, sh);
      if (built < DURATION) timer = setTimeout(topUp, 0);
    }, 0);
  }

  return {
    stop() {
      if (stopped) return;
      stopped = true;
      if (timer !== undefined) clearTimeout(timer);
      const now = ac.currentTime;
      kill.gain.cancelScheduledValues(now);
      kill.gain.setValueAtTime(1, now);
      kill.gain.linearRampToValueAtTime(0, now + 0.03);
      for (const s of m.sources) {
        try {
          s.stop(now + 0.035);
        } catch {
          // Never started.
        }
      }
      const release = () => {
        try {
          kill.disconnect();
        } catch {
          // Already disconnected.
        }
      };
      setTimeout(release, 120);
    },
  };
}
