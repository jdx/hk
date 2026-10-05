// The words on screen, in the showreel's caption band (type.ts) and its
// motion, on the song's clock. A line's call is the shantyman's: it lands
// whole just before he sings it, one word per 1/32 note, in paper. The
// crew's answer is hk's, in cyan on the row below, and lands word by word
// as the crew sings each one. Both wipe away left to right as the next line
// starts to land, and the next line writes in behind the wipe.

import { PALETTE } from "../bible";
import { clamp, progress, swiftOut } from "../math";
import { drawText, drawWords, entrance, font, layout, MONO, WIPE, WORD, type WordStyle, wordStyle } from "../type";
import { answerAt, callAt, type Lyric } from "./song";

/** The call: paper, 64 px, the largest size at which the longest line fits the stage. */
export const CALL: WordStyle = wordStyle(64);
/** The answer: hk's cyan, its code too. */
export const ANSWER: WordStyle = { ...wordStyle(64, PALETTE.cyan), code: { font: font(58, 600, MONO), fill: PALETTE.cyan } };

/** Lines start 160 px in, as captions do. */
export const LYRIC_X = 160;
/** The call's baseline, one row above the answer's. */
export const CALL_Y = 852;
/** The answer's baseline: the captions' lower one, clear of a player's controls. */
export const ANSWER_Y = 936;
/** The widest a line may be: the stage, x 160 to 1760. */
export const LYRIC_WIDTH = 1600;
/** A call has landed this long before its first word is sung. */
export const LEAD = 0.1;
/**
 * The crew answer at the end of a line, often just as the next line starts
 * to land, so an answer stays up at least this long after its last word.
 */
export const ANSWER_HOLD = 0.75;

/** A row of a line on the song's clock: its words, its baseline, and when its wipe starts. */
interface TimedRow {
  text: string;
  y: number;
  out: number;
}

/** A line placed on the song's clock. */
export interface TimedLyric {
  id: string;
  /** The call lands whole, its last word at `land`. */
  call?: TimedRow & { land: number };
  /** The answer lands a word at a time, at `lands`. */
  answer?: TimedRow & { lands: readonly number[] };
}

/** How one line is shown, where the picture needs it shown otherwise. */
export interface LyricPlace {
  /** The picture shows the line itself (the end card types `mise use hk`): no caption. */
  hidden?: boolean;
  /** The call's baseline, px. */
  y?: number;
  /** When the wipe starts, seconds, if not as the next line lands. */
  out?: number;
}

/** When a call's first word starts to rise. */
const callEntrance = (l: Lyric): number => (l.call ? entrance(l.call, callAt(l) - LEAD) : answerAt(l)[0] - WORD);

/**
 * Every shown line on the song's clock: each wipes away as the next shown
 * line starts to land, unless `place` says otherwise.
 */
export function timeLyrics(lyrics: readonly Lyric[], place: Readonly<Record<string, LyricPlace>> = {}): TimedLyric[] {
  const shown = lyrics.filter((l) => !place[l.id]?.hidden);
  return shown.map((l, i) => {
    const p = place[l.id] ?? {};
    const next = shown[i + 1];
    const out = p.out ?? (next ? callEntrance(next) : Infinity);
    const lands = answerAt(l);
    return {
      id: l.id,
      call: l.call ? { text: l.call, land: callAt(l) - LEAD, y: p.y ?? CALL_Y, out } : undefined,
      answer: l.answer ? { text: l.answer, lands, y: ANSWER_Y, out: Math.max(out, lands[lands.length - 1] + ANSWER_HOLD) } : undefined,
    };
  });
}

/** When a line first shows. */
export const lyricStart = (l: TimedLyric): number =>
  Math.min(l.call ? entrance(l.call.text, l.call.land) : Infinity, l.answer ? l.answer.lands[0] - WORD : Infinity);

/** When a line has wiped away. */
export const lyricEnd = (l: TimedLyric): number => Math.max(l.call?.out ?? -Infinity, l.answer?.out ?? -Infinity) + WIPE;

interface Run {
  text: string;
  code: boolean;
}

/** A line's words, each a list of runs, with backticks marking code (as type.ts reads them). */
function words(text: string): Run[][] {
  const out: Run[][] = [];
  let runs: Run[] = [];
  let run = "";
  let code = false;
  const endRun = () => {
    if (run) runs.push({ text: run, code });
    run = "";
  };
  for (const ch of text) {
    if (ch === "`") {
      endRun();
      code = !code;
    } else if (ch === " ") {
      endRun();
      if (runs.length) out.push(runs);
      runs = [];
    } else {
      run += ch;
    }
  }
  endRun();
  if (runs.length) out.push(runs);
  return out;
}

/** A line's width in `style`, words spaced as prose (drawWords's spacing). */
export function lineWidth(ctx: CanvasRenderingContext2D, text: string, style: WordStyle): number {
  const space = layout(ctx, " ", style.font).width;
  const list = words(text);
  let width = 0;
  list.forEach((w, i) => {
    for (const r of w) width += layout(ctx, r.text, r.code ? style.code.font : style.font).width;
    if (i < list.length - 1) width += space;
  });
  return width;
}

/**
 * `text` with each word landing at its own time, `lands[i]` (the last time
 * for any words beyond them): rising and fading in over the 1/32 note
 * before it, as drawWords's words do.
 */
export function drawTimedWords(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  style: WordStyle,
  lands: readonly number[],
  t: number,
  align: "left" | "center" = "left",
): void {
  const space = layout(ctx, " ", style.font).width;
  let wx = align === "center" ? x - lineWidth(ctx, text, style) / 2 : x;
  words(text).forEach((w, i) => {
    const land = lands[Math.min(i, lands.length - 1)];
    const p = progress(land - WORD, land, t);
    let rx = wx;
    for (const r of w) {
      const spec = r.code ? style.code : style;
      if (p > 0) {
        ctx.save();
        ctx.globalAlpha *= clamp(p / 0.6);
        drawText(ctx, r.text, rx, y + style.rise * (1 - swiftOut(p)), { font: spec.font, fill: spec.fill });
        ctx.restore();
      }
      rx += layout(ctx, r.text, spec.font).width;
    }
    wx = rx + space;
  });
}

/** How far a wipe starting at `out` has reached across a line `width` wide from x. */
function wipeEdge(ctx: CanvasRenderingContext2D, x: number, width: number, style: WordStyle, t: number, out: number): number {
  const k = progress(out, out + WIPE, t);
  if (k <= 0) return -Infinity;
  const pad = 0.1 * layout(ctx, "M", style.font).width;
  return x - pad + k * (width + 2 * pad);
}

/** Draw whichever lines are up at `t`. */
export function drawLyrics(ctx: CanvasRenderingContext2D, t: number, lyrics: readonly TimedLyric[]): void {
  const rows = ({ call, answer }: TimedLyric) => [
    ...(call ? [{ row: call, style: CALL, draw: () => drawWords(ctx, call.text, LYRIC_X, call.y, CALL, t, call.land) }] : []),
    ...(answer ? [{ row: answer, style: ANSWER, draw: () => drawTimedWords(ctx, answer.text, LYRIC_X, answer.y, ANSWER, answer.lands, t) }] : []),
  ];
  // A row wiping away on a baseline clips a newer row on it to the left of
  // the wipe's edge, so the new words write in behind it.
  const edges = new Map<number, number>();
  for (const l of lyrics) {
    for (const { row, style } of rows(l)) {
      if (t >= row.out && t < row.out + WIPE) edges.set(row.y, wipeEdge(ctx, LYRIC_X, lineWidth(ctx, row.text, style), style, t, row.out));
    }
  }
  for (const l of lyrics) {
    if (t < lyricStart(l) || t >= lyricEnd(l)) continue;
    for (const { row, style, draw } of rows(l)) {
      if (t >= row.out + WIPE) continue;
      const behind = t < row.out ? edges.get(row.y) : undefined;
      if (behind === -Infinity) continue;
      ctx.save();
      if (behind !== undefined) {
        ctx.beginPath();
        ctx.rect(behind - 2e5, -1e5, 2e5, 2e5);
        ctx.clip();
      }
      const own = wipeEdge(ctx, LYRIC_X, lineWidth(ctx, row.text, style), style, t, row.out);
      if (own > -Infinity) {
        ctx.beginPath();
        ctx.rect(own, -1e5, 2e5, 2e5);
        ctx.clip();
      }
      draw();
      ctx.restore();
    }
  }
}
