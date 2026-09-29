<script setup lang="ts">
import { useShantyMode } from "./useShantyMode";

// What every switch shares, rendered once: the screen reader's announcement
// after a flip, and the ship that sails across the foot of the screen when
// the mode turns on.
const { status, sailing, landed } = useShantyMode();
</script>

<template>
  <p class="visually-hidden" role="status">{{ status }}</p>
  <div v-if="sailing" class="hk-ship" aria-hidden="true" @animationend.self="landed">
    <svg viewBox="0 0 56 48" fill="currentColor">
      <path d="M28 4v31" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M30 7 49 31H30Z" />
      <path d="M26 11 9 31h17Z" opacity=".75" />
      <path d="M28 4l9 3-9 3Z" />
      <path d="M5 34h46l-8 10H13Z" />
    </svg>
  </div>
</template>

<style scoped>
/* Off screen until it sails, so a browser without animations shows nothing.
   Below the announcement banner, which sits at 1001. */
.hk-ship {
  position: fixed;
  bottom: 14vh;
  left: 0;
  z-index: 40;
  width: 56px;
  color: var(--hk-sea-brass);
  pointer-events: none;
  transform: translateX(-100px);
  animation: hk-sail 3.2s linear forwards;
}
.hk-ship svg {
  display: block;
  width: 100%;
  height: auto;
  animation: hk-bob 0.8s ease-in-out 4 alternate;
}
@keyframes hk-sail {
  to {
    transform: translateX(calc(100vw + 100px));
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
