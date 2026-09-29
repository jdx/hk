<script setup lang="ts">
import { withBase } from "vitepress";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { data } from "../benchmarks.data";
import { data as shantyFiles } from "../shanty.data";
import { data as showreel } from "../showreel.data";
import { reelKind } from "./shanty-mode";
import { describeChapters } from "./showreel/describe";
import { factsFromBenchmarks, races } from "./showreel/facts";

// The reel is rendered to MP4 files by `mise run docs:showreel` (the docs
// deploy runs it), so this is a plain video player. A build leaves the section
// out when it has no render to show: no showreel, or with sea shanty mode on,
// neither video. Only facts.ts, describe.ts and timeline.ts are imported here:
// they draw nothing, so they are safe to server-render.

// `shanty` is set on the pirate landing page (sea shanty mode), which plays
// the music video and links to pirate pages.
const props = defineProps<{ shanty?: boolean }>();
const to = (path: string) => withBase(props.shanty ? `/pirate${path}` : path);

const facts = factsFromBenchmarks(data);
const described = describeChapters(facts);
// The race scene draws timings only when the benchmark backs a claim;
// without one, the caption points at how hk runs steps instead.
const timed = races(facts).length > 0;

const SHOWREEL_LABEL =
  "hk showreel, about a minute long: what hk does when you run git commit. The fixers from hk.pkl run in parallel, even on the same file, with file locks keeping them from colliding, fixes are staged while your unstaged work is kept, and a commit that cannot be fixed is blocked. Chapters are listed below.";
const SHANTY_LABEL =
  "Bound for the Main, the music video for the hk sea shanty, about three and a half minutes long. The sung words are on screen, and the lyrics are on the sea shanty page.";

// What the player plays: the showreel, or with sea shanty mode on, the
// shanty's music video (shanty-mode.ts). Each is rendered at deploy, so a
// build can have either, both or neither, and the player never offers one that
// is not there. The pirate landing page is its own page, so the server
// renders it with the music video already in place.
const reel = computed(() => {
  const kind = reelKind(props.shanty === true, { showreel: showreel !== null, video: shantyFiles.video !== null });
  if (kind === "shanty" && shantyFiles.video) {
    return {
      kind,
      src: shantyFiles.video.src,
      poster: shantyFiles.video.poster,
      chapters: "/bound-for-the-main-chapters.vtt",
      section: "Music video",
      label: SHANTY_LABEL,
      play: "Play Bound for the Main",
    } as const;
  }
  if (kind === "showreel" && showreel) {
    return {
      kind,
      src: showreel.src,
      poster: showreel.poster,
      chapters: "/showreel-chapters.vtt",
      section: "Showreel",
      label: SHOWREEL_LABEL,
      play: "Play the hk showreel",
    } as const;
  }
  return null;
});

// The page is served with the 60 fps file, which plays everywhere. Once it is
// mounted, and before anyone presses play, it switches to the 120 fps file if
// the browser says it decodes that smoothly and power-efficiently (in
// practice, in hardware). This tests the decoder, not the display, so a
// capable 60 Hz screen gets the larger file too. Nothing downloads until play.
// The probe only records the answer; the showreel is the only reel with a 120
// fps file, so the answer cannot reach the music video whenever it resolves.
const player = ref<HTMLVideoElement>();
const prefer120 = ref(false);
const src = computed(() =>
  reel.value?.kind === "showreel" && prefer120.value && showreel?.video120 ? showreel.video120.src : (reel.value?.src ?? ""),
);

// The native controls keep a fixed height while the video shrinks, so on a
// phone or a small tablet they cover the poster's caption ("Can't be fixed?
// hk blocks the commit."). Until someone starts the reel, the player shows
// its own play button instead, over the middle of the showreel's frame, which
// holds no text that must be read (the music video's poster does; see
// .is-shanty). The server-rendered page keeps the controls, so the player
// works without JavaScript; they go once the page is hydrated.
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

// Switching reels replaces the <video> (its :key), so the new one starts from
// its poster with the play button. The old one is stopped and emptied first,
// or its stream would keep downloading until the element is collected.
watch(
  () => reel.value?.kind,
  () => {
    const old = player.value;
    if (old) {
      old.pause();
      old.removeAttribute("src");
      old.load();
    }
    started.value = false;
  },
  { flush: "pre" },
);

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
    if (smooth && powerEfficient && idle) prefer120.value = true;
  } catch {
    // Older browsers reject the query; they keep the 60 fps file.
  }
});
</script>

<template>
  <section v-if="reel" class="hk-showreel" :aria-label="reel.section">
    <figure>
      <div class="hk-showreel-stage">
        <!-- No autoplay, and nothing downloads until someone presses play. -->
        <video
          ref="player"
          :key="reel.kind"
          :src="withBase(src)"
          :poster="withBase(reel.poster)"
          width="1920"
          height="1080"
          :controls="!hydrated || started"
          playsinline
          preload="none"
          :aria-label="reel.label"
          @play="started = true"
        >
          <!-- Generated from the reel's sections; see showreel/timeline.ts. -->
          <track kind="chapters" srclang="en" label="Chapters" :src="withBase(reel.chapters)" default />
        </video>
        <!-- The whole frame is the target; the glyph sits in its middle, or for the music video below its subtitle. -->
        <button
          v-if="hydrated && !started"
          type="button"
          class="hk-showreel-play"
          :class="{ 'is-shanty': reel.kind === 'shanty' }"
          :aria-label="reel.play"
          @click="play"
        >
          <svg viewBox="0 0 64 64" aria-hidden="true">
            <circle cx="32" cy="32" r="30" />
            <path d="M26 20.5v23L44.5 32z" />
          </svg>
        </button>
      </div>
      <ol v-if="reel.kind === 'showreel'" class="sr-only" aria-label="Showreel chapters">
        <li v-for="c in described" :key="c.id">{{ c.label }}: {{ c.text }}</li>
      </ol>
      <figcaption v-if="reel.kind === 'showreel'">
        What hk does when you commit, in about a minute. The captions are on
        screen, so it works with the sound off.
        <template v-if="timed">
          Timings come from the
          <a :href="to('/benchmarks')">benchmarks</a>.
        </template>
        <a v-else :href="to('/why-hk')">How execution works →</a>
      </figcaption>
      <figcaption v-else>
        Bound for the Main, the hk sea shanty: a commit's voyage through hk,
        sung by the crew. Turn the sound on; the words are on screen.
        <a :href="to('/shanty')">Read the lyrics →</a>
        ·
        <a :href="withBase(shantyFiles.song)">Download the MP3</a> (3:32).
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
/* The music video's poster holds its title in the middle of the frame, so its
   glyph sits below the subtitle, at the foot of the frame; the poster's faint
   swells may run behind it on a narrow frame. The padding is a share of the
   frame's width: 41% puts the glyph's middle at about 80% of the frame's
   height on a wide screen and 84% on a phone. On the narrowest frames the
   44 px glyph would run past the bottom edge, so the padding stops 48 px short
   of the frame's height (56.25% of its width). ShantyVideo.vue does the same. */
.hk-showreel-play.is-shanty {
  place-items: start center;
  padding-top: min(41%, calc(56.25% - 48px));
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
