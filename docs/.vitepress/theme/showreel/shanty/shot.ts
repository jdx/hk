// A shot: one stretch of the music video's picture, on the song's clock,
// and the adapter that plays a showreel scene as one. video.ts lists them.

import { BEAT, type LitRect, type Scene, type SceneEnv, type SectionId } from "../bible";
import { clamp, keys, type Key } from "../math";
import { scenes } from "../scenes";

/** One stretch of the picture, on the song's clock (seconds). */
export interface Shot {
  /** What it shows, for previews and tests. */
  name: string;
  start: number;
  /** The frame at `end` is the next shot's. */
  end: number;
  /** Paint the whole frame at song time `t`. */
  draw(ctx: CanvasRenderingContext2D, t: number, env: SceneEnv): void;
  /** The lit screen the vignette spares, as a reel scene reports it. */
  lit?(t: number): LitRect | null;
  /** Cross-fade in from the shot before over this many seconds from `start`. */
  fade?: number;
}

const byId = new Map<SectionId, Scene>(scenes.map((s) => [s.id, s]));

/** The showreel's scene for section `id`. */
export function reelScene(id: SectionId): Scene {
  const scene = byId.get(id);
  if (!scene) throw new Error(`no showreel scene "${id}"`);
  return scene;
}

/**
 * A showreel scene's local time, seconds, at song time `t`: `map` keys song
 * time to the scene's own beats (beat 0 is its first frame), eased linearly
 * between keys and held beyond them.
 */
export function sceneClock(scene: Scene, map: readonly Key[]): (t: number) => number {
  // Time runs forward, and so does the scene: it may hold, never rewind.
  map.forEach(([t, b], i) => {
    if (i && !(t > map[i - 1][0] && b >= map[i - 1][1])) throw new Error(`${scene.id}: key ${i} (${t} s, beat ${b}) runs backwards`);
  });
  const beatOf = keys(map);
  const len = scene.end - scene.start;
  return (t) => clamp(beatOf(t) * BEAT, 0, len - 1e-6);
}

/** Draw `scene` at local time `lt`, on the reel's clock as the reel would. */
export function drawScene(ctx: CanvasRenderingContext2D, scene: Scene, lt: number, env: SceneEnv): void {
  scene.draw(ctx, lt, { ...env, t: scene.start + lt, facts: null });
}

/**
 * A showreel scene as a shot, its moments keyed to the words that tell
 * them (sceneClock): stash's lid shuts on "cold", catch's hook tugs on
 * "tugged" and on "twice". The scene is given the reel's global time for
 * its local time, as the reel gives it, and no benchmark facts. `over`
 * draws on top of it, on the song's clock.
 */
export function reelShot(
  id: SectionId,
  start: number,
  end: number,
  map: readonly Key[],
  o: { fade?: number; over?: (ctx: CanvasRenderingContext2D, t: number) => void } = {},
): Shot {
  const scene = reelScene(id);
  const local = sceneClock(scene, map);
  return {
    name: id,
    start,
    end,
    fade: o.fade,
    draw(ctx, t, env) {
      drawScene(ctx, scene, local(t), env);
      if (o.over) {
        ctx.save();
        o.over(ctx, t);
        ctx.restore();
      }
    },
    lit: (t) => scene.lit?.(local(t), { facts: null }) ?? null,
  };
}
