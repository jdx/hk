// A stand-in for a section whose scene is not built yet: the section's name
// and what the viewer should learn from it, and a beat ruler with a playhead
// and the caption spans, so the section's captions and timing can be
// reviewed in place. Each scene file is one of these until its real scene
// replaces it; the reel draws the captions themselves over it.
//
// A stub starts on the handoff frame it inherits (handoff.ts handoffIn) and
// ends on the one it owes the next section (handoffOut), lit screen and all,
// so the reel stays seamless on every bar line while real scenes replace
// stubs one by one (test/handoff-frames.test.ts holds every scene to it): it
// fades from the first to its placeholder over a beat and a half, fades the
// second in over the second-to-last beat, and holds it for the last. Across
// the whip (everywhere|race) the placeholder whips out, or in, under the
// speed lines instead.

import { BEAT, PALETTE, type ReelFacts, type Scene, type SceneEnv, SECTIONS, type SectionId, sec } from "../bible";
import { rgba } from "../color";
import { roundedRect } from "../fx";
import { handoffIn, handoffOut } from "../handoff";
import { smoothstep } from "../math";
import { type Caption, DETAIL, drawText, entrance, font, LABEL, layout, MONO } from "../type";
import { drawWhip, drawWhipIn, drawWhipOut, WHIP_END, WHIP_START } from "../whip";

/** A section's captions: fixed, or chosen by the facts. */
export type Captions = readonly Caption[] | ((facts: ReelFacts | null) => readonly Caption[]);

// The picture area: everything above the captions' band.
const FX = 160;
const FY = 72;
const FW = 1600;
const FH = 640;
const PAD = 48;
const RULER_Y = FY + FH - 76;

/** Beats the stub takes to leave its first handoff frame. */
const FADE_IN = 1.5;

/** Break `text` into lines no wider than `width` in `spec`. */
function wrap(ctx: CanvasRenderingContext2D, text: string, spec: string, width: number): string[] {
  const lines: string[] = [];
  let line = "";
  for (const word of text.split(" ")) {
    const next = line ? `${line} ${word}` : word;
    if (line && layout(ctx, next, spec).width > width) {
      lines.push(line);
      line = word;
    } else {
      line = next;
    }
  }
  if (line) lines.push(line);
  return lines;
}

/** The section's beats, bar numbers on the reel's count, the caption spans, and the playhead. */
function ruler(ctx: CanvasRenderingContext2D, id: SectionId, caps: readonly Caption[], lt: number): void {
  const s = sec(id);
  const beats = s.bars * 4;
  const x0 = FX + PAD;
  const pb = (FW - 2 * PAD) / beats;
  const firstBar = Math.round(s.start / (4 * BEAT)) + 1;
  ctx.fillStyle = rgba(PALETTE.paper, 0.3);
  for (let k = 0; k <= beats; k++) {
    const onBar = k % 4 === 0;
    ctx.fillRect(x0 + k * pb - 1, RULER_Y - (onBar ? 18 : 8), 2, onBar ? 36 : 16);
    if (onBar && k < beats) {
      drawText(ctx, `bar ${firstBar + k / 4}`, x0 + k * pb + 8, RULER_Y + 40, {
        font: font(20, 400, MONO),
        fill: PALETTE.text3,
      });
    }
  }
  // Each caption line from its first word to the start of its wipe, the
  // landing marked.
  for (const c of caps) {
    c.lines.forEach((l, i) => {
      const a = entrance(l.text, l.in * BEAT) / BEAT;
      const y = RULER_Y - 44 - (c.lines.length - 1 - i) * 12;
      ctx.fillStyle = rgba(PALETTE.paper, 0.35);
      ctx.fillRect(x0 + a * pb, y, (c.out - a) * pb, 6);
      ctx.fillStyle = PALETTE.paper;
      ctx.fillRect(x0 + l.in * pb - 1, y - 3, 3, 12);
    });
  }
  ctx.fillStyle = PALETTE.warm;
  ctx.fillRect(x0 + (lt / BEAT) * pb - 1.5, RULER_Y - 64, 3, 88);
  drawText(ctx, `b ${(lt / BEAT).toFixed(2)} / ${beats}`, FX + FW - PAD, FY + 76, {
    font: font(28, 400, MONO),
    fill: PALETTE.warm,
    align: "right",
  });
}

function placeholder(ctx: CanvasRenderingContext2D, id: SectionId, learns: string, caps: readonly Caption[], lt: number): void {
  const n = SECTIONS.findIndex((s) => s.id === id) + 1;
  ctx.save();
  ctx.strokeStyle = rgba(PALETTE.paper, 0.22);
  ctx.lineWidth = 2;
  ctx.setLineDash([12, 10]);
  roundedRect(ctx, FX, FY, FW, FH, 20);
  ctx.stroke();
  ctx.restore();

  const top = FY + 76;
  drawText(ctx, String(n).padStart(2, "0"), FX + PAD, top, { font: font(40, 700, MONO), fill: PALETTE.warm });
  drawText(ctx, sec(id).label, FX + PAD + 76, top, { font: LABEL.font, fill: PALETTE.paper });
  drawText(ctx, `${id} · placeholder`, FX + PAD + 76, top + 44, { font: font(24, 400, MONO), fill: PALETTE.text3 });
  const spec = DETAIL.font;
  wrap(ctx, learns, spec, FW - 2 * PAD).forEach((line, i) => {
    drawText(ctx, line, FX + PAD, top + 120 + i * 52, { font: spec, fill: PALETTE.text2 });
  });
  ruler(ctx, id, caps, lt);
}

/**
 * A placeholder scene for section `id` with its storyboard captions, which
 * starts and ends exactly on its handoff frames. `learns` is what the viewer
 * should take from the section.
 */
export function stubScene(id: SectionId, captions: Captions = [], learns = ""): Scene {
  const S = sec(id);
  const from = handoffIn(id);
  const to = handoffOut(id);
  const caps = (facts: ReelFacts | null): readonly Caption[] => (typeof captions === "function" ? captions(facts) : captions);
  const whipsOut = to?.meet === "motion";
  const whipsIn = from?.meet === "motion";
  const beats = S.len / BEAT;
  // How far the placeholder has replaced the inherited frame, and how far
  // the owed frame has come up over it (a motion handoff is met by the move
  // itself instead).
  const leave = (lt: number): number => (from && !whipsIn ? smoothstep(0, FADE_IN * BEAT, lt) : 1);
  const arrive = (lt: number): number => (to && !whipsOut ? smoothstep((beats - 2) * BEAT, (beats - 1) * BEAT, lt) : 0);
  return {
    id: S.id,
    start: S.start,
    end: S.end,
    draw(ctx, lt, env: SceneEnv) {
      const fill = () => {
        ctx.fillStyle = PALETTE.bg;
        ctx.fillRect(0, 0, env.W, env.H);
      };
      const card = () => placeholder(ctx, id, learns, caps(env.facts), lt);
      fill();
      if (whipsOut && env.t >= WHIP_START) {
        // Smeared copies of the placeholder, then the speed lines over them.
        drawWhipOut(ctx, env.t, card);
        drawWhip(ctx, env.t);
        return;
      }
      if (whipsIn && env.t < WHIP_END) {
        drawWhipIn(ctx, env.t, card);
        drawWhip(ctx, env.t);
        return;
      }
      // The body: the inherited frame giving way to the placeholder.
      const k = leave(lt);
      if (from && k < 1) from.draw(ctx, env);
      if (k > 0) {
        ctx.save();
        ctx.globalAlpha = k;
        if (k < 1) fill();
        card();
        ctx.restore();
      }
      // The owed frame fades in over the second-to-last beat and holds.
      const b = arrive(lt);
      if (to && b > 0) {
        ctx.save();
        ctx.globalAlpha = b;
        to.draw(ctx, env);
        ctx.restore();
      }
    },
    // A handoff's lit screen, as far as its frame shows, so the vignette
    // meets the real scene on the other side of the bar line.
    lit(lt) {
      const b = arrive(lt);
      if (to?.lit && b > 0) return { ...to.lit, alpha: to.lit.alpha * b };
      const k = leave(lt);
      if (from?.lit && k < 1) return { ...from.lit, alpha: from.lit.alpha * (1 - k) };
      return null;
    },
    captions: caps,
  };
}
