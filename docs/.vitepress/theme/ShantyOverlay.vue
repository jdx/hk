<script setup lang="ts">
import { useShantyMode } from "./useShantyMode";

// What every switch shares, rendered once: the screen reader's announcement
// after a flip, and the brig that sails across the foot of the screen when
// the mode turns on, drawn in the brass of the chart with its wake behind.
const { status, sailing, landed } = useShantyMode();
</script>

<template>
  <p class="visually-hidden" role="status">{{ status }}</p>
  <div v-if="sailing" class="hk-ship" aria-hidden="true" @animationend.self="landed">
    <svg viewBox="0 0 132 100">
      <g class="hk-ship-hull">
        <path d="M44 8h2.4v62H44zM78 14h2.4v56H78z" />
        <path d="M46.4 8.4l12 3.6-12 3.6Z" />
        <path d="M80.4 14.4l9 2.8-9 2.8Z" />
        <path d="M30 18c7 2 21 2 30 0-2 5-2 10 0 15-9 2-23 2-30 0 2-5 2-10 0-15Z" opacity=".9" />
        <path d="M26 37c9 2.6 27 2.6 38 0-2.6 6.4-2.6 13 0 19.4-11 2.6-29 2.6-38 0 2.6-6.4 2.6-13 0-19.4Z" />
        <path d="M67 24c6 1.8 16 1.8 24 0-1.6 4.4-1.6 8.8 0 13.2-8 1.8-18 1.8-24 0 1.6-4.4 1.6-8.8 0-13.2Z" opacity=".9" />
        <path d="M64 41c8 2.2 22 2.2 30 0-2 5.6-2 11 0 16.6-8 2.2-22 2.2-30 0 2-5.6 2-11 0-16.6Z" />
        <path d="M82 16l30 44H82Z" opacity=".75" />
        <path d="M8 66h98l14-8-4 10c-3 10-10 16-20 16H26C16 84 10 78 8 66Z" />
      </g>
      <g class="hk-ship-lines" fill="none" stroke-linecap="round">
        <path d="M106 66l14-8M45 8 8 66M79 14l37 44" opacity=".6" />
        <path d="M2 92c6-4 12-4 18 0s12 4 18 0 12-4 18 0M40 97c5-3 10-3 15 0s10 3 15 0" opacity=".7" />
      </g>
      <g class="hk-ship-ports">
        <circle cx="34" cy="73" r="2.2" />
        <circle cx="48" cy="73" r="2.2" />
        <circle cx="62" cy="73" r="2.2" />
        <circle cx="76" cy="73" r="2.2" />
        <circle cx="90" cy="73" r="2.2" />
      </g>
    </svg>
  </div>
</template>

<style scoped>
/* Off screen until it sails, so a browser without animations shows nothing.
   Below the announcement banner, which sits at 1001. */
.hk-ship {
  position: fixed;
  bottom: 12vh;
  left: 0;
  z-index: 40;
  width: 104px;
  color: var(--hk-sea-brass);
  pointer-events: none;
  transform: translateX(-140px);
  animation: hk-sail 3.6s linear forwards;
}
.hk-ship svg {
  display: block;
  width: 100%;
  height: auto;
  overflow: visible;
  transform-origin: 50% 80%;
  animation: hk-bob 0.9s ease-in-out 4 alternate;
}
.hk-ship-hull {
  fill: currentColor;
}
.hk-ship-lines {
  stroke: currentColor;
  stroke-width: 1.4;
}
.hk-ship-ports {
  fill: var(--vp-c-bg);
}
@keyframes hk-sail {
  to {
    transform: translateX(calc(100vw + 140px));
  }
}
@keyframes hk-bob {
  from {
    transform: rotate(-3deg);
  }
  to {
    transform: rotate(3deg);
  }
}
</style>
