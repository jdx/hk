import type { Scene } from "../bible";
import { scene as catchScene } from "./catch";
import { scene as commit } from "./commit";
import { scene as config } from "./config";
import { scene as end } from "./end";
import { scene as everywhere } from "./everywhere";
import { scene as lanes } from "./lanes";
import { scene as morph } from "./morph";
import { scene as open } from "./open";
import { scene as race } from "./race";
import { scene as restore } from "./restore";
import { scene as stash } from "./stash";

/** One scene per section, in the timeline's order (timeline.ts SECTIONS). */
export const scenes: Scene[] = [open, config, commit, stash, lanes, restore, catchScene, everywhere, race, morph, end];
