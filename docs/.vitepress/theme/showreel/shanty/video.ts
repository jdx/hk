// The music video for "Bound for the Main": the song's own story told with
// the showreel's scenes, each played on the song's clock, between the
// chorus, title and outro shots drawn for it, with the words in the
// captions' band. Like the reel, `render` is a pure function of time, so
// the renderer (shanty-video.mjs) can draw any frame on its own.

import { H, PALETTE, type SceneEnv, W } from "../bible";
import { grain, makeCanvas, vignette } from "../fx";
import { clamp } from "../math";
import { chorus } from "./chorus";
import { drawDance } from "./dance";
import { drawLyrics, timeLyrics } from "./lyrics";
import { outro } from "./outro";
import { reelShot, type Shot } from "./shot";
import { bar, LYRICS, SONG, sungAt } from "./song";
import { title } from "./title";

export type { Shot } from "./shot";

/** Where each line is shown, where the picture shows it itself. */
export const PLACE = {
  // Typed into the end card's install box as it is sung.
  "outro.3": { hidden: true },
  // The end card's address slot, until the address lands in it.
  "outro.4": { y: 880, out: 204.4 },
  "outro.2": { out: 193.2 },
} as const;

/** A chorus cuts in on its first "Heave". */
const heave = (id: string): number => sungAt(`${id}.1`, "Heave") - 0.08;

/**
 * Where the picture cuts, on the song's clock: on a bar line, on the beat
 * before a line whose first word the picture shows, or on a chorus's first
 * "Heave". Each shot runs from its cut to the next.
 */
export const CUT = {
  commit: title.end,
  config: bar(10),
  "chorus-1": heave("chorus-1"),
  stash: bar(22),
  lanes: bar(30),
  "chorus-2": heave("chorus-2"),
  restore: bar(45.5),
  "chorus-3": heave("chorus-3"),
  bridge: bar(61.5),
  "bridge-lanes": bar(64),
  "bridge-main": bar(65),
  "verse-5": bar(65.75),
  charts: bar(70),
  "verse-6": sungAt("verse-6.1", "Her") - 0.25,
  "final-chorus": heave("final-chorus"),
  outro: outro.start,
} as const;

/** The shot list, in order. */
export const SHOTS: readonly Shot[] = [
  title,
  // Verse 1: `git commit` slams in on "I christened", the message flips in
  // as it is sung, and Enter lands on "return".
  reelShot("commit", CUT.commit, CUT.config, [
    [CUT.commit, 0.8],
    [sungAt("verse-1.1", "I"), 1],
    [sungAt("verse-1.1", "christened"), 1.5],
    [sungAt("verse-1.1", "her"), 2],
    [sungAt("verse-1.1", "sails"), 4.25],
    [16.4, 4.8],
    [18.7, 4.84],
    [sungAt("verse-1.2", "return"), 5.05],
    [19.45, 5.5],
    [20.3, 6.5],
    [CUT.config, 7],
  ]),
  // All hands: hk's builtins wash in as rivers from stem to stern, and the
  // commit's steps are plucked into hk.pkl.
  reelShot("config", CUT.config, CUT["chorus-1"], [
    [CUT.config, 0.6],
    [sungAt("verse-1.3", "hands"), 1.3],
    [sungAt("verse-1.4", "tumbled"), 3.6],
    [sungAt("verse-1.4", "stern"), 5.8],
    [CUT["chorus-1"], 6.7],
  ]),
  chorus("chorus-1", { from: CUT["chorus-1"], to: CUT.stash }),
  // Verse 2: the TODO line peels into the stash, and the lid shuts on "cold".
  reelShot("stash", CUT.stash, CUT.lanes, [
    [CUT.stash, 0.4],
    [sungAt("verse-2.1", "staged"), 1.6],
    [sungAt("verse-2.2", "So"), 1.95],
    [sungAt("verse-2.2", "stashed"), 2.5],
    [sungAt("verse-2.2", "cold"), 3],
    [sungAt("verse-2.3", "crew"), 3.5],
    [sungAt("verse-2.3", "ship"), 5],
    [sungAt("verse-2.4", "line"), 7.5],
    [sungAt("verse-2.4", "hold"), 9.4],
    [CUT.lanes, 12],
  ]),
  // Verse 3: every step starts on the crew's "Heave", and the padlocks
  // shut on "made fast".
  reelShot("lanes", CUT.lanes, CUT["chorus-2"], [
    [CUT.lanes, 0.05],
    [sungAt("verse-3.1", "Heave"), 1],
    [bar(38), 16],
  ]),
  chorus("chorus-2", { from: CUT["chorus-2"], to: CUT.restore }),
  // Verse 4: the fixes are staged, so the dance is struck out as it is sung;
  // the stash comes up like an anchor; the commit lands on main.
  reelShot("restore", CUT.restore, CUT["chorus-3"], [
    [CUT.restore, 0.1],
    [sungAt("verse-4.1", "staged"), 1.5],
    [sungAt("verse-4.1", "dance"), 1.95],
    [sungAt("verse-4.3", "Then"), 2],
    [sungAt("verse-4.3", "stash"), 2.9],
    [sungAt("verse-4.3", "up"), 3.4],
    [sungAt("verse-4.3", "weighed"), 3.75],
    [sungAt("verse-4.3", "Haul"), 4.6],
    [sungAt("verse-4.4", "feat"), 5.75],
    [sungAt("verse-4.4", "made"), 6.4],
    [sungAt("verse-4.4", "main"), 7],
    [CUT["chorus-3"], 8.8],
  ], { over: drawDance }),
  chorus("chorus-3", { from: CUT["chorus-3"], to: CUT.bridge }),
  // The bridge: check shows the diff and fix applies it as each is called
  // for; all hands at once; bound for the main.
  reelShot("everywhere", CUT.bridge, CUT["bridge-lanes"], [
    [CUT.bridge, 6.1],
    [sungAt("bridge.1", "cargo"), 6.5],
    [sungAt("bridge.1", "Aitch-kay"), 7],
    [sungAt("bridge.2", "canvas"), 7.5],
    [sungAt("bridge.2", "Aitch-kay"), 8.2],
    [sungAt("bridge.2", "fix"), 8.875],
    [CUT["bridge-lanes"], 9.5],
  ]),
  reelShot("lanes", CUT["bridge-lanes"], CUT["bridge-main"], [
    [CUT["bridge-lanes"], 0.9],
    [sungAt("bridge.3", "All"), 1.1],
    [CUT["bridge-main"], 5],
  ]),
  reelShot("restore", CUT["bridge-main"], CUT["verse-5"], [
    [CUT["bridge-main"], 6.3],
    [sungAt("bridge.4", "main"), 7],
    [CUT["verse-5"], 7.9],
  ]),
  // Verse 5: the hook, the helm and the harbor-master are the commit, the
  // terminal and CI, each landing as it is named, with one set of steps;
  // the charts are hk.pkl.
  reelShot("everywhere", CUT["verse-5"], CUT.charts, [
    [CUT["verse-5"], 0],
    [sungAt("verse-5.1", "hook"), 0.125],
    [sungAt("verse-5.1", "helm"), 1],
    [sungAt("verse-5.1", "harbor-master"), 2],
    [sungAt("verse-5.1", "Heave"), 3],
    [sungAt("verse-5.2", "crew"), 5],
    [CUT.charts, 5.45],
  ]),
  reelShot("config", CUT.charts, CUT["verse-6"], [
    [CUT.charts, 7],
    [sungAt("verse-5.3", "set"), 7.8],
    [sungAt("verse-5.4", "aitch-kay"), 9],
    [sungAt("verse-5.4", "typed"), 9.6],
    [CUT["verse-6"], 11.2],
  ]),
  // Verse 6: deploy.sh sails for main, shellcheck sings out as she stops on
  // the ✗, the hook snags her and tugs twice, and heaves her off the line.
  reelShot("catch", CUT["verse-6"], CUT["final-chorus"], [
    [CUT["verse-6"], 0.3],
    [sungAt("verse-6.1", "sailed"), 1],
    [sungAt("verse-6.1", "Bound"), 2.2],
    [sungAt("verse-6.2", "And"), 2.5],
    [sungAt("verse-6.3", "Then"), 2.8],
    [sungAt("verse-6.3", "Bug"), 3],
    [sungAt("verse-6.4", "Dead"), 3.5],
    [sungAt("verse-6.4", "hook"), 4],
    [sungAt("verse-6.4", "tugged"), 4.5],
    [sungAt("verse-6.4", "twice"), 5],
    [167.4, 5.2],
    [sungAt("verse-6.5", "Heave"), 5.3],
    [sungAt("verse-6.5", "Ho"), 6.3],
    [CUT["final-chorus"], 6.6],
  ]),
  chorus("final-chorus", { from: CUT["final-chorus"], to: CUT.outro }),
  outro,
];

/** The shot on at `t`. */
export function shotAt(t: number): Shot {
  for (const s of SHOTS) if (t < s.end) return s;
  return SHOTS[SHOTS.length - 1];
}

/** Every captioned line on the song's clock. */
export const TIMED = timeLyrics(LYRICS, PLACE);

let fadeLayer: HTMLCanvasElement | null = null;

export interface Video {
  duration: number;
  render(ctx: CanvasRenderingContext2D, t: number, pw: number, ph: number): void;
}

/** The whole video. `raw` leaves out the words, grain and vignette. */
export function createVideo(o: { raw?: boolean } = {}): Video {
  return {
    duration: SONG.duration,
    render(ctx, time, pw, ph) {
      const t = clamp(time, 0, SONG.duration - 1e-6);
      const env: SceneEnv = { W, H, t, facts: null };
      const reset = () => {
        ctx.setTransform(pw / W, 0, 0, ph / H, 0, 0);
        ctx.globalAlpha = 1;
        ctx.globalCompositeOperation = "source-over";
      };
      reset();
      ctx.fillStyle = PALETTE.bg;
      ctx.fillRect(0, 0, W, H);
      const i = SHOTS.indexOf(shotAt(t));
      const s = SHOTS[i];
      const fading = i > 0 && s.fade && t < s.start + s.fade ? clamp((t - s.start) / s.fade) : 1;
      if (fading < 1) {
        // The shot before, whole, then this one over it through a layer.
        ctx.save();
        SHOTS[i - 1].draw(ctx, t, env);
        ctx.restore();
        const { width, height } = ctx.canvas;
        fadeLayer ??= makeCanvas(width, height);
        if (fadeLayer.width !== width || fadeLayer.height !== height) {
          fadeLayer.width = width;
          fadeLayer.height = height;
        }
        const l = fadeLayer.getContext("2d")!;
        l.setTransform(pw / W, 0, 0, ph / H, 0, 0);
        l.globalAlpha = 1;
        l.globalCompositeOperation = "source-over";
        l.save();
        s.draw(l, t, env);
        l.restore();
        ctx.save();
        ctx.setTransform(1, 0, 0, 1, 0, 0);
        ctx.globalAlpha = fading;
        ctx.drawImage(fadeLayer, 0, 0);
        ctx.restore();
      } else {
        ctx.save();
        s.draw(ctx, t, env);
        ctx.restore();
      }
      reset();
      if (!o.raw) {
        vignette(ctx, W, H, 0.35, s.lit?.(t) ?? null);
        drawLyrics(ctx, t, TIMED);
        grain(ctx, W, H, t, 0.07);
      }
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },
  };
}

/** The poster: the title, whole, before the first word lands. */
export const POSTER_TIME = 11.9;

export { resetTypeCache } from "../type";
