// The reel's soundtrack, synthesized with the Web Audio API: oscillators,
// seeded noise, filters, and envelopes, with no samples and no network. The
// renderer (showreel-video.mjs) plays it into an OfflineAudioContext and
// muxes it into the MP4. The whole score is written in reel seconds and can
// start at any point in the reel: from there it plays the same sounds, on
// the same samples, as a render from the top (test/score.test.ts).
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
//
// The master is mixed for delivery as AAC: about -17 LUFS integrated, with
// a lookahead limiter and a soft ceiling that keep the true peak near -2.3
// dBTP, so the encode's overshoot stays under -1 dBTP.

import type { ReelFacts } from "./facts";
import { arc, compose } from "./score";
import { type Dip, floats, type Floats, LATENCY, Mix, type Pt, roomOf, shared } from "./score/mix";
import { GAP } from "./score/morph";
import { DURATION, sec } from "./timeline";

/** The groove's buses under the effects, before the loudness arc (score/index.ts) rides them. */
const DRUMS = 0.6;
const MUSIC = 0.78;

/** Gain from the glue compressor into the limiter. */
const MAKEUP = 2.35;
/** The limiter's threshold, dBFS: peaks above it are held down before the soft ceiling. */
const LIMIT = -4.5;

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

/** Play a whole-reel dip curve on `p`, joining it at reel time `t0`. */
function dipCurve(m: Mix, p: AudioParam, t0: number, vals: Floats): void {
  const k = Math.min(vals.length - 2, Math.ceil(t0 / DIP_STEP - 1e-9));
  const rest = vals.slice(k);
  p.setValueCurveAtTime(rest, m.at(k * DIP_STEP), (rest.length - 1) * DIP_STEP);
}

/**
 * The two duck curves, from the accents and the boots the score recorded as
 * it was built. The facts can move the benchmark race's cues, and their
 * ducks with them.
 */
function ducks(m: Mix): { drums: Floats; music: Floats } {
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
 * at context time `when`, on a context that has not started rendering. Pass
 * the same `facts` the picture draws, so the race's cues land on its bars.
 */
export function playScore(ac: BaseAudioContext, dest: AudioNode, from: number, when: number, facts: ReelFacts | null = null): void {
  if (!(from < DURATION - 0.03)) return;
  const sh = shared(ac);

  // Master: a DC blocker, a touch less sub and more air, a 16 kHz lowpass,
  // a glue compressor, makeup, a limiter, a soft ceiling, the end fade, and
  // the breath before the resolve.
  const pre = ac.createGain();
  // The staccato bass and the boots' falling thuds are gated low tones, each
  // leaving a little DC and subsonic rumble; a microphone's coupling would
  // take it out, so this does, below anything a speaker plays.
  const dc = ac.createBiquadFilter();
  dc.type = "highpass";
  dc.frequency.value = 18;
  dc.Q.value = -3;
  // The bass and the boots' thuds carried half the mix's energy below 120
  // Hz, where laptop and phone speakers play little of it and it only fed
  // the limiter; their grit and the planks carry them above it. The air is
  // only a lift: the tambourine and the clicks already carry the top, and
  // more of it tires the ear on earbuds.
  const low = ac.createBiquadFilter();
  low.type = "lowshelf";
  low.frequency.value = 100;
  low.gain.value = -3;
  const air = ac.createBiquadFilter();
  air.type = "highshelf";
  air.frequency.value = 7000;
  air.gain.value = 1;
  // Nothing above 16 kHz: the AAC encode drops it anyway, and a click's
  // top octave removed after the limiter comes back as overshoot. Two
  // Butterworth sections, 24 dB an octave.
  const tops = [0.54, 1.31].map((q) => {
    const f = ac.createBiquadFilter();
    f.type = "lowpass";
    f.frequency.value = 16000;
    // Q here is resonance in dB: the two sections' Butterworth Qs.
    f.Q.value = 20 * Math.log10(q);
    return f;
  });
  const comp = ac.createDynamicsCompressor();
  comp.threshold.value = -18;
  comp.knee.value = 10;
  comp.ratio.value = 2;
  comp.attack.value = 0.005;
  const makeup = ac.createGain();
  makeup.gain.value = MAKEUP;
  const limiter = ac.createDynamicsCompressor();
  limiter.threshold.value = LIMIT;
  limiter.knee.value = 0;
  limiter.ratio.value = 20;
  limiter.attack.value = 0;
  limiter.release.value = 0.12;
  const trim = ac.createGain();
  // Undo the limiter's automatic makeup (the spec's 0.6 power of the gain
  // its curve applies at full scale), and halve, because the ceiling
  // curve's domain is ±2.
  trim.gain.value = 0.5 * 10 ** ((0.6 * LIMIT * (1 - 1 / 20)) / 20);
  const ceiling = ac.createWaveShaper();
  ceiling.curve = sh.ceiling;
  const tail = ac.createGain();
  const breath = ac.createGain();
  pre.connect(dc).connect(low).connect(air).connect(tops[0]).connect(tops[1]).connect(comp).connect(makeup).connect(limiter).connect(trim).connect(ceiling).connect(tail).connect(breath).connect(dest);

  const sfx = ac.createGain();
  sfx.connect(pre);
  const drums = ac.createGain();
  const drumDuck = ac.createGain();
  drums.connect(drumDuck).connect(pre);
  const music = ac.createGain();
  const musicDuck = ac.createGain();
  music.connect(musicDuck).connect(pre);

  const verb = ac.createGain();
  const conv = ac.createConvolver();
  conv.buffer = roomOf(ac, sh);
  const verbLow = ac.createBiquadFilter();
  verbLow.type = "highpass";
  verbLow.frequency.value = 220;
  const verbOut = ac.createGain();
  verbOut.gain.value = 2.5;
  const verbGate = ac.createGain();
  verb.connect(conv).connect(verbLow).connect(verbOut).connect(verbGate).connect(pre);

  const m = new Mix(ac, sh, from, when, { sfx, drums, music }, verb);
  // The loudness arc rides the groove's two buses, section by section.
  const fader = arc();
  m.set(drums.gain, fader.map(([t, v]): Pt => [t, DRUMS * v]));
  m.set(music.gain, fader.map(([t, v]): Pt => [t, MUSIC * v]));
  // Every voice, recording the accents and boots the ducks follow.
  compose(m, facts);
  const p = ducks(m);
  const start = m.floor;

  // A fresh compressor starts clamped down and takes ~200 ms to open; a fast
  // release until just after the first sound lets it settle at once.
  comp.release.value = 0.001;
  comp.release.setValueAtTime(0.2, m.at(start) + 0.03);
  dipCurve(m, drumDuck.gain, start, p.drums);
  dipCurve(m, musicDuck.gain, start, p.music);
  // The breath before the resolve: the room goes quiet with everything else.
  const resolve = sec("end").start;
  // It closes all the way by the time the riser cuts, so the sixteenth is
  // not the room's tail, and opens again only on the downbeat, where the
  // resolve covers what is left of the tail.
  m.set(verbGate.gain, [[GAP - 0.02, 1], [GAP - 0.002, 0.01, "exp"], [GAP + 0.002, 0], [resolve, 0], [resolve + 0.004, 1]]);
  // The cut leaves the master's filters settling for a few tens of
  // milliseconds (a subsonic drift under -54 dBFS). After the limiter, on the
  // output's own clock, the whole sixteenth is digital silence: the gate
  // takes the riser's last 3 ms, already 45 dB down, and opens on the downbeat.
  m.set(breath.gain, [[GAP - 0.003, 1], [GAP, 0], [resolve - 0.001, 0], [resolve, 1]], from, LATENCY);
  // Everything, reverb included, is silent before the last frame.
  const end = DURATION;
  m.set(tail.gain, [[end - 0.4, 1], [end - 0.14, 0.25, "exp"], [end - 0.035, 0.0002, "exp"], [end - 0.028, 0]]);
}
