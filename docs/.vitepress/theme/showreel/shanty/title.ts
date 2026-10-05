// The title, over the song's opening boots and claps, up to the half bar
// before the shantyman's first word. The showreel's `open` inks the
// wordmark on the beat as the hook is lowered to it, from the first
// downbeat (bar 0) to bar 2; the song's name then lands under the mark a
// word a beat, the line about it a bar later, and the sea rises at the foot
// of the frame. The first frame fades up from night over the two pickups.

import { BEAT, PALETTE } from "../bible";
import { smoothstep } from "../math";
import { scene as open } from "../scenes/open";
import { drawWords, wordStyle } from "../type";
import { drawTimedWords } from "./lyrics";
import { drawSea } from "./sea";
import { bar, beatAt, FIRST_BAR, SONG, timeOfBeat } from "./song";
import type { Shot } from "./shot";

/** The name: paper, 120 px, a little under the mark. */
const NAME = { text: SONG.title, style: wordStyle(120), y: 676 };
/** The line under it. */
const ABOUT = { text: "A sea shanty about hk", style: wordStyle(48, PALETTE.text2), y: 760 };

/** The wordmark's local time: open's beats are the song's from bar 0, held after. */
const openAt = (t: number): number => Math.min(Math.max(0, beatAt(t) - FIRST_BAR) * BEAT, open.end - open.start - 1e-6);

/** When the name's words land: on the four beats of bar 2. */
const NAME_LANDS = [0, 1, 2, 3].map((k) => timeOfBeat(FIRST_BAR + 8 + k));

export const title: Shot = {
  name: "title",
  start: 0,
  end: bar(5.5),
  draw(ctx, t, env) {
    const lt = openAt(t);
    open.draw(ctx, lt, { ...env, t: open.start + lt });
    drawSea(ctx, t, { rise: smoothstep(bar(3), bar(4), t) });
    drawTimedWords(ctx, NAME.text, 960, NAME.y, NAME.style, NAME_LANDS, t, "center");
    drawWords(ctx, ABOUT.text, 960, ABOUT.y, ABOUT.style, t, bar(3), Infinity, "center");
    // Up from night over the pickups.
    const dark = 1 - smoothstep(0.1, bar(0), t);
    if (dark > 0) {
      ctx.save();
      ctx.globalAlpha = dark;
      ctx.fillStyle = PALETTE.night;
      ctx.fillRect(0, 0, env.W, env.H);
      ctx.restore();
    }
  },
};
