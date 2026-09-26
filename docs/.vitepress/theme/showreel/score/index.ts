// The score, section by section. Each section's share lives in its own
// module named for its section id, written from the section's origin
// (`sec(id)`), so lengthening or inserting a section moves its sounds with
// it. Where a scene times its motion with a formula, its module imports that
// formula or anchor from the scene, so the sound lands where the picture
// does. The race's cues also take the facts, as its bars do.

import type { ReelFacts } from "../facts";
import { BEAT, SECTIONS, type Section, type SectionId, sec } from "../timeline";
import { part as catchPart } from "./catch";
import { part as commit } from "./commit";
import { part as config } from "./config";
import { part as end } from "./end";
import { part as everywhere } from "./everywhere";
import { part as lanes } from "./lanes";
import type { Mix, Pt } from "./mix";
import { part as morph } from "./morph";
import { part as open } from "./open";
import { part as race } from "./race";
import { part as restore } from "./restore";
import { part as stash } from "./stash";

/** One layer of a section's share: the mix, the section, and the facts the picture draws. */
type Layer = (m: Mix, s: Section, facts: ReelFacts | null) => void;

/** One section's share of the score. Every layer is optional. */
export interface Part {
  /**
   * The groove's fader for the section (the drums and music buses; the cues
   * keep their own levels), 1 by default. Together the sections' levels are
   * the reel's loudness arc: the groove enters softly, the lanes are the
   * heart, and the race is the peak.
   */
  level?: number;
  /** Sound design: a sound for each accent the section's picture choreographs. */
  cues?: Layer;
  /** The stomp, and the boots the bass and the concertina pump with. */
  drums?: Layer;
  bass?: Layer;
  /** The concertina's tune. */
  lead?: Layer;
  /** The chords: the concertina's, the fiddle's chops, and pads. */
  pads?: Layer;
}

/** Every section's part. A new section needs an entry, even an empty one. */
export const PARTS: Record<SectionId, Part> = {
  open,
  config,
  commit,
  stash,
  lanes,
  restore,
  catch: catchPart,
  everywhere,
  race,
  morph,
  end,
};

/**
 * The groove's fader over the whole reel, from each section's level: it
 * moves over the half beat before a bar line, so the new level arrives
 * with the downbeat.
 */
export function arc(): Pt[] {
  const pts: Pt[] = [[0, PARTS[SECTIONS[0].id].level ?? 1]];
  for (let i = 1; i < SECTIONS.length; i++) {
    const from = PARTS[SECTIONS[i - 1].id].level ?? 1;
    const to = PARTS[SECTIONS[i].id].level ?? 1;
    if (from === to) continue;
    const t = sec(SECTIONS[i].id).start;
    pts.push([t - BEAT / 2, from], [t, to]);
  }
  return pts;
}

/** The layers, built in this order: the effects first, then the music under them. */
const LAYERS = ["cues", "drums", "bass", "lead", "pads"] as const;

/** Build one pass of the whole score into `m`. */
export function compose(m: Mix, facts: ReelFacts | null = null): void {
  for (const layer of LAYERS) {
    for (const { id } of SECTIONS) PARTS[id][layer]?.(m, sec(id), facts);
  }
}
