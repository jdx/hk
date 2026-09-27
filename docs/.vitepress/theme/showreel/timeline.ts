// The reel's clock: the tempo and the timeline of sections. It imports
// nothing, so the landing page can read the chapters without the drawing
// code. bible.ts re-exports all of it.

/**
 * 120 BPM puts a 4/4 bar in exactly 2 seconds, so every bar line and every
 * 32nd note lands on a frame at both 60 and 120 fps.
 */
export const BPM = 120;
export const BEAT = 60 / BPM;
export const BAR = BEAT * 4;
export const beat = (n: number): number => n * BEAT;
export const bar = (n: number): number => n * BAR;

/**
 * The timeline: every section in order, in whole bars. The reel's length,
 * the chapters, each scene's span, and where the score places its cues all
 * come from here, so lengthening or inserting a section moves everything
 * after it. Labels are what a viewer sees in the player's chapter menu and
 * the page's chapter list.
 */
export const SECTIONS = [
  { id: "open", label: "hk", bars: 2 },
  { id: "config", label: "Steps in hk.pkl", bars: 3 },
  { id: "commit", label: "git commit runs hk", bars: 3 },
  { id: "stash", label: "Unstaged work is stashed", bars: 3 },
  { id: "lanes", label: "Steps in parallel", bars: 4 },
  { id: "restore", label: "Fixes staged, edits back", bars: 3 },
  { id: "catch", label: "A commit hk can't fix", bars: 3 },
  { id: "everywhere", label: "Commit, terminal, CI", bars: 3 },
  { id: "race", label: "Benchmarks", bars: 4 },
  { id: "morph", label: "Morph", bars: 1 },
  { id: "end", label: "mise use hk", bars: 3 },
] as const satisfies readonly { id: string; label: string; bars: number }[];

export type SectionId = (typeof SECTIONS)[number]["id"];

/** One section on the reel's clock. Times are global seconds. */
export interface Section {
  id: SectionId;
  label: string;
  bars: number;
  start: number;
  /** The frame at `end` belongs to the next section. */
  end: number;
  /** Length in seconds. */
  len: number;
  /** Global time of local time `lt`, seconds into the section. */
  at(lt: number): number;
  /** Global time of beat `n` of the section; beat 0 is its first downbeat. */
  beat(n: number): number;
  /** Global time of bar `n` of the section. */
  bar(n: number): number;
}

const TIMELINE = new Map<SectionId, Section>();
{
  // Counted in whole bars and beats, so every boundary is exact.
  let first = 0;
  for (const { id, label, bars } of SECTIONS) {
    const b0 = first;
    TIMELINE.set(id, {
      id,
      label,
      bars,
      start: bar(b0),
      end: bar(b0 + bars),
      len: bar(bars),
      at: (lt) => bar(b0) + lt,
      beat: (n) => beat(b0 * 4 + n),
      bar: (n) => bar(b0 + n),
    });
    first += bars;
  }
}

/** Where section `id` sits on the reel's clock. */
export function sec(id: SectionId): Section {
  const s = TIMELINE.get(id);
  if (!s) throw new Error(`no section "${id}"`);
  return s;
}

/** The whole reel: every section, end to end. */
export const DURATION = bar(SECTIONS.reduce((n, s) => n + s.bars, 0));

/** One chapter per section, for players and the page's chapter list. */
export const CHAPTERS = SECTIONS.map(({ id }) => {
  const { label, start, end } = sec(id);
  return { id, label, start, end };
});

/** `h:mm:ss.fff`, a WebVTT timestamp. Section bounds are whole milliseconds. */
function vttTime(s: number): string {
  const ms = Math.round(s * 1000);
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(Math.floor(ms / 3_600_000))}:${pad(Math.floor(ms / 60_000) % 60)}:${pad(Math.floor(ms / 1000) % 60)}.${pad(ms % 1000, 3)}`;
}

/**
 * The chapters as a WebVTT chapters track, one cue per section, identified
 * by its id. docs/public/showreel-chapters.vtt is this text; a test keeps it
 * current.
 */
export function chaptersVtt(): string {
  const cues = CHAPTERS.map((c) => `${c.id}\n${vttTime(c.start)} --> ${vttTime(c.end)}\n${c.label}\n`);
  return ["WEBVTT\n", ...cues].join("\n");
}
