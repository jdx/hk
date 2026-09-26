<script setup lang="ts">
import { withBase } from "vitepress";
import { nextTick, onMounted, ref } from "vue";
import { data } from "../benchmarks.data";
import { data as showreel } from "../showreel.data";
import { describeChapters } from "./showreel/describe";
import { factsFromBenchmarks, races } from "./showreel/facts";

// The reel is rendered to MP4 files by `mise run docs:showreel` (the docs
// deploy runs it), so this is a plain video player. Builds without a render
// leave the section out. Only facts.ts, describe.ts and timeline.ts are
// imported here: they draw nothing, so they are safe to server-render.

const facts = factsFromBenchmarks(data);
const described = describeChapters(facts);
// The race scene draws timings only when the benchmark backs a claim;
// without one, the caption points at how hk runs steps instead.
const timed = races(facts).length > 0;

// The page is served with the 60 fps file, which plays everywhere. Once it is
// mounted, and before anyone presses play, it switches to the 120 fps file if
// the browser says it decodes that smoothly and power-efficiently (in
// practice, in hardware). This tests the decoder, not the display, so a
// capable 60 Hz screen gets the larger file too. Nothing downloads until play.
const player = ref<HTMLVideoElement>();
const src = ref(showreel?.src ?? "");

// The native controls keep a fixed height while the video shrinks, so on a
// phone or a small tablet they cover the poster's caption ("Can't be fixed?
// hk blocks the commit."). Until someone starts the reel, the player shows
// its own play button instead, over the middle of the frame, which holds no
// text that must be read. The server-rendered page keeps the controls, so
// the player works without JavaScript; they go once the page is hydrated.
const hydrated = ref(false);
const started = ref(false);
function play() {
  started.value = true;
  const video = player.value;
  if (!video) return;
  // A failed start leaves the controls up to show it.
  video.play().catch(() => {});
  // The button is gone; keep keyboard focus on the player.
  void nextTick(() => video.focus({ preventScroll: true }));
}

onMounted(async () => {
  hydrated.value = true;
  const video120 = showreel?.video120;
  if (!video120 || !navigator.mediaCapabilities) return;
  try {
    const { smooth, powerEfficient } = await navigator.mediaCapabilities.decodingInfo({
      type: "file",
      video: {
        // H.264 High at level 5.1, as the renderer encodes it.
        contentType: 'video/mp4; codecs="avc1.640033"',
        width: 1920,
        height: 1080,
        framerate: 120,
        bitrate: video120.bitrate,
      },
    });
    // Someone who already pressed play keeps the file that is playing.
    const idle = player.value?.paused && player.value.readyState === HTMLMediaElement.HAVE_NOTHING;
    if (smooth && powerEfficient && idle) src.value = video120.src;
  } catch {
    // Older browsers reject the query; they keep the 60 fps file.
  }
});
</script>

<template>
  <section v-if="showreel" class="hk-showreel" aria-label="Showreel">
    <figure>
      <div class="hk-showreel-stage">
        <!-- No autoplay, and nothing downloads until someone presses play. -->
        <video
          ref="player"
          :src="withBase(src)"
          :poster="withBase(showreel.poster)"
          width="1920"
          height="1080"
          :controls="!hydrated || started"
          playsinline
          preload="none"
          aria-label="hk showreel, about a minute long: what hk does when you run git commit. Steps from hk.pkl run in parallel, fixes are staged while your unstaged work is kept, and a commit that cannot be fixed is blocked. Chapters are listed below."
          @play="started = true"
        >
          <!-- Generated from the reel's sections; see showreel/timeline.ts. -->
          <track kind="chapters" srclang="en" label="Chapters" :src="withBase('/showreel-chapters.vtt')" default />
        </video>
        <!-- The whole frame is the target; the glyph sits in its middle. -->
        <button
          v-if="hydrated && !started"
          type="button"
          class="hk-showreel-play"
          aria-label="Play the hk showreel"
          @click="play"
        >
          <svg viewBox="0 0 64 64" aria-hidden="true">
            <circle cx="32" cy="32" r="30" />
            <path d="M26 20.5v23L44.5 32z" />
          </svg>
        </button>
      </div>
      <ol class="sr-only" aria-label="Showreel chapters">
        <li v-for="c in described" :key="c.id">{{ c.label }}: {{ c.text }}</li>
      </ol>
      <figcaption>
        What hk does when you commit, in about a minute. The captions are on
        screen, so it works with the sound off.
        <template v-if="timed">
          Timings come from the
          <a :href="withBase('/benchmarks')">benchmarks</a>.
        </template>
        <a v-else :href="withBase('/why-hk')">How execution works →</a>
      </figcaption>
    </figure>
  </section>
</template>

<style scoped>
/* The hero above keeps its 72 px bottom padding; the principles below bring
   their own border and top padding. */
.hk-showreel {
  padding: 0 0 56px;
}
figure {
  margin: 0;
}
/* Framed like the hero's .hk-preview card. The reel is one dark stage in both
   site themes, so the element's own background is the reel's night colour:
   no white or page-coloured box shows before the poster paints. */
video {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 16 / 9;
  background: #071019;
  border: 1px solid var(--vp-c-divider);
  border-radius: 10px;
  box-shadow: 0 20px 60px rgb(0 0 0 / 8%);
}
/* An 8% black shadow vanishes on the dark page. */
.dark video {
  box-shadow:
    0 30px 80px -40px #000,
    0 14px 32px -22px rgb(0 0 0 / 70%);
}
video:focus-visible,
.hk-showreel-play:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 4px;
}
.hk-showreel-stage {
  position: relative;
}
/* Covers the video exactly, so a click anywhere on the poster plays it. */
.hk-showreel-play {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  border-radius: 10px;
  cursor: pointer;
}
/* Drawn in the reel's own colours, since it sits on the reel's night stage
   in both site themes. 10% of the frame's width (44 to 96 px) keeps it in
   the frame's middle, clear of the caption band below. */
.hk-showreel-play svg {
  width: clamp(44px, 10%, 96px);
  height: auto;
  filter: drop-shadow(0 6px 18px rgb(0 0 0 / 45%));
  transition: transform 0.15s ease;
}
.hk-showreel-play circle {
  fill: rgb(7 16 25 / 62%);
  stroke: #f4eee3;
  stroke-width: 2.5;
  transition:
    fill 0.15s ease,
    stroke 0.15s ease;
}
.hk-showreel-play path {
  fill: #f4eee3;
}
.hk-showreel-play:hover svg {
  transform: scale(1.06);
}
.hk-showreel-play:hover circle,
.hk-showreel-play:focus-visible circle {
  fill: rgb(7 16 25 / 80%);
  stroke: #4adef0;
}
@media (prefers-reduced-motion: reduce) {
  .hk-showreel-play svg,
  .hk-showreel-play circle {
    transition: none;
  }
  .hk-showreel-play:hover svg {
    transform: none;
  }
}
figcaption {
  margin-top: 14px;
  color: var(--vp-c-text-2);
  font-size: 13px;
  line-height: 1.6;
}
figcaption a {
  color: var(--vp-c-brand-1);
}
figcaption a:hover {
  text-decoration: underline;
  text-underline-offset: 3px;
}
.sr-only {
  border: 0;
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
  height: 1px;
  margin: -1px;
  overflow: hidden;
  padding: 0;
  position: absolute;
  white-space: nowrap;
  width: 1px;
}
/* The stacked page's 32 px rhythm, as .hk-principles uses there. */
@media (max-width: 719px) {
  .hk-showreel {
    padding-bottom: 32px;
  }
}
</style>
