// The outro, over the boots alone. The showreel's `open` again: the hook is
// lowered to ink the wordmark as the shantyman calls the hands aboard
// (bars 94 to 96), and the sea comes in under it. As the three words come,
// the mark crosses to the end card's place and the stage darkens to night,
// its barb drawing back into the point, so the card starts from its own
// first frame (morph|end) and clicks the barb out again. The card types
// `mise use hk` as it is sung, and its install box lights when the crew
// echo it; the last line takes the address's place (video.ts) until it has
// been sung, and then the address lands. The card holds still as the last
// chord rings, and the frame fades to night with it.

import { PALETTE } from "../bible";
import { mix, rgba } from "../color";
import { glow, roundedRect } from "../fx";
import { drawLogo, LOGO_END, LOGO_OPEN } from "../kit/logo";
import { inOutCubic, lerp, progress, pulse, smoothstep } from "../math";
import { BOX } from "../scenes/end";
import { drawSea } from "./sea";
import { drawScene, reelScene, sceneClock, type Shot } from "./shot";
import { bar, SONG, sungAt } from "./song";

const open = reelScene("open");
const end = reelScene("end");

/** The mark is inked on the beat, as in the title. */
const openAt = sceneClock(open, [
  [bar(94), 0],
  [bar(96), 8],
]);

/** The mark crosses to the card's place over the half bar before bar 98. */
export const CROSS = [bar(97.75), bar(98)] as const;

/** When the crew echo `mise use hk`. */
const ECHO = sungAt("outro.3", "Meez", 1);

/**
 * The card on the song's clock: its copy arrives over the first bar, its
 * keys (end.ts T_KEY0 to the `k` on b5.125) on "mise use hk" as sung, and it
 * waits before the address (b6) until the last line has wiped away.
 */
const endAt = sceneClock(end, [
  [CROSS[1], 0],
  [sungAt("outro.3", "Meez"), 3.875],
  [sungAt("outro.3", "aitch-kay") + 0.17, 5.125],
  [195.3, 5.6],
  [204.6, 5.8],
  [205.2, 6],
  [206.4, 8],
  [207.8, 10],
]);

/** Night falls as the last chord rings out. */
const DUSK = [208.4, SONG.duration - 0.05] as const;

export const outro: Shot = {
  name: "outro",
  start: bar(94),
  end: SONG.duration,
  draw(ctx, t, env) {
    if (t < CROSS[0]) {
      drawScene(ctx, open, openAt(t), env);
      drawSea(ctx, t, { rise: smoothstep(bar(94), bar(95), t), alpha: 1 - smoothstep(CROSS[0] - 0.4, CROSS[0], t) });
    } else if (t < CROSS[1]) {
      const k = inOutCubic(progress(CROSS[0], CROSS[1], t));
      ctx.fillStyle = mix(PALETTE.bg, PALETTE.night, k);
      ctx.fillRect(0, 0, env.W, env.H);
      const place = { cx: lerp(LOGO_OPEN.cx, LOGO_END.cx, k), cy: lerp(LOGO_OPEN.cy, LOGO_END.cy, k), h: lerp(LOGO_OPEN.h, LOGO_END.h, k) };
      drawLogo(ctx, place, [1, 1, 1, 1, 1, 1 - k]);
    } else {
      drawScene(ctx, end, endAt(t), env);
      const echo = pulse(t, ECHO, 0.04, 0.45);
      if (echo > 0.01) {
        ctx.save();
        glow(ctx, BOX.x + BOX.w / 2, BOX.y + BOX.h / 2, 360, PALETTE.cyan, 0.35 * echo);
        roundedRect(ctx, BOX.x, BOX.y, BOX.w, BOX.h, BOX.r);
        ctx.strokeStyle = rgba(PALETTE.cyanBright, echo);
        ctx.lineWidth = 3;
        ctx.stroke();
        ctx.restore();
      }
    }
    const dark = smoothstep(DUSK[0], DUSK[1], t);
    if (dark > 0) {
      ctx.save();
      ctx.globalAlpha = dark;
      ctx.fillStyle = PALETTE.night;
      ctx.fillRect(0, 0, env.W, env.H);
      ctx.restore();
    }
  },
  lit: (t) => (t >= CROSS[1] ? (end.lit?.(endAt(t), { facts: null }) ?? null) : null),
};
