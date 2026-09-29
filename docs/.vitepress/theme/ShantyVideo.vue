<script setup lang="ts">
import { withBase } from "vitepress";
import { nextTick, onMounted, ref } from "vue";
import { data } from "../shanty.data";

// The shanty's music video, rendered to an MP4 by `mise run docs:shanty` (the
// docs deploy runs it), so this is a plain player; builds without a render
// play the song on its own instead. Nothing downloads until play.

const player = ref<HTMLVideoElement>();

// As on the landing page's showreel, the native controls keep a fixed height
// while the video shrinks, so on a phone they cover the poster's title. Until
// someone starts it, the player shows its own play button instead; the
// server-rendered page keeps the controls, so it plays without JavaScript.
const hydrated = ref(false);
const started = ref(false);
function play() {
  started.value = true;
  const video = player.value;
  if (!video) return;
  // A failed start leaves the controls up to show it.
  video.play().catch(() => {});
  void nextTick(() => video.focus({ preventScroll: true }));
}

onMounted(() => {
  hydrated.value = true;
});
</script>

<template>
  <figure class="hk-shanty">
    <div v-if="data.video" class="hk-shanty-stage">
      <video
        ref="player"
        :src="withBase(data.video.src)"
        :poster="withBase(data.video.poster)"
        width="1920"
        height="1080"
        :controls="!hydrated || started"
        playsinline
        preload="none"
        aria-label="Bound for the Main, the music video, about three and a half minutes long. The sung words are on screen, and printed below."
        @play="started = true"
      >
        <!-- Generated from the song's sections; see showreel/shanty/song.ts. -->
        <track kind="chapters" srclang="en" label="Chapters" :src="withBase('/bound-for-the-main-chapters.vtt')" default />
      </video>
      <button
        v-if="hydrated && !started"
        type="button"
        class="hk-shanty-play"
        aria-label="Play Bound for the Main"
        @click="play"
      >
        <svg viewBox="0 0 64 64" aria-hidden="true">
          <circle cx="32" cy="32" r="30" />
          <path d="M26 20.5v23L44.5 32z" />
        </svg>
      </button>
    </div>
    <audio
      v-else
      controls
      preload="none"
      :src="withBase(data.song)"
      aria-label="Bound for the Main, the song"
    >
      <a :href="withBase(data.song)">Download the song</a>.
    </audio>
    <figcaption>
      <template v-if="data.video">
        Each verse plays its part of the
        <a :href="withBase('/')">showreel</a> in time with the song, and the
        words are on screen.
      </template>
      <a :href="withBase(data.song)">Download the song</a> (MP3, 3:32).
    </figcaption>
  </figure>
</template>

<style scoped>
.hk-shanty {
  margin: 24px 0 8px;
}
.hk-shanty-stage {
  position: relative;
}
/* One dark stage in both site themes, framed as the landing page's showreel. */
video {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 16 / 9;
  background: #071019;
  border: 1px solid var(--vp-c-divider);
  border-radius: 10px;
}
video:focus-visible,
.hk-shanty-play:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 4px;
}
audio {
  display: block;
  width: 100%;
}
/* The poster holds the title in the middle of the frame, so the glyph sits
   below the subtitle, at the foot of the frame. The padding is a share of the
   frame's width, stopped 48 px short of the frame's height on the narrowest
   frames so the glyph stays inside; HomeShowreel.vue's music video does the
   same. */
.hk-shanty-play {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: start center;
  padding-top: min(41%, calc(56.25% - 48px));
  border-radius: 10px;
  cursor: pointer;
}
.hk-shanty-play svg {
  width: clamp(44px, 10%, 96px);
  height: auto;
  filter: drop-shadow(0 6px 18px rgb(0 0 0 / 45%));
  transition: transform 0.15s ease;
}
.hk-shanty-play circle {
  fill: rgb(7 16 25 / 62%);
  stroke: #f4eee3;
  stroke-width: 2.5;
  transition:
    fill 0.15s ease,
    stroke 0.15s ease;
}
.hk-shanty-play path {
  fill: #f4eee3;
}
.hk-shanty-play:hover svg {
  transform: scale(1.06);
}
.hk-shanty-play:hover circle,
.hk-shanty-play:focus-visible circle {
  fill: rgb(7 16 25 / 80%);
  stroke: #4adef0;
}
@media (prefers-reduced-motion: reduce) {
  .hk-shanty-play svg,
  .hk-shanty-play circle {
    transition: none;
  }
  .hk-shanty-play:hover svg {
    transform: none;
  }
}
figcaption {
  margin-top: 12px;
  color: var(--vp-c-text-2);
  font-size: 13px;
  line-height: 1.6;
}
figcaption a {
  color: var(--vp-c-brand-1);
}
</style>
