<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { data as shantyFiles } from "../shanty.data";
import { data as showreel } from "../showreel.data";
import { announcement, reelKind, TURNING_CLASS } from "./shanty-mode";

// The landing page's switch for sea shanty mode, in the place mise puts its
// announcement chip: above the title. It owns no state. It asks HomePage to
// flip it, which paints <html> and remembers the choice (see shanty-mode.ts).

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

// The server-rendered page has no switch to press: it is space kept open, and
// it appears once the page can act on a press, so nothing dead is on screen
// and nothing moves when it arrives.
const hydrated = ref(false);
onMounted(() => {
  hydrated.value = true;
});

// What a screen reader hears after the flip, cleared a few seconds later.
const status = ref("");
let statusTimer: ReturnType<typeof setTimeout> | undefined;
function announce(on: boolean) {
  clearTimeout(statusTimer);
  const hasVideo = reelKind(true, { showreel: showreel !== null, video: shantyFiles.video !== null }) === "shanty";
  status.value = announcement(on, hasVideo);
  statusTimer = setTimeout(() => {
    status.value = "";
  }, 4000);
}

// The anchor drops into its disc once, when the mode turns on. It is a state of
// its own, not a look of `.is-on`: that would replay whenever hover or focus
// left the chip, and lose to the hover sway at the moment of switching on.
const dropping = ref(false);
let dropTimer: ReturnType<typeof setTimeout> | undefined;
function drop() {
  dropping.value = true;
  clearTimeout(dropTimer);
  dropTimer = setTimeout(() => {
    dropping.value = false;
  }, 800);
}

// The ship sails across the foot of the screen once, after the mode turns on.
const sailing = ref(false);
let shipTimer: ReturnType<typeof setTimeout> | undefined;
function sail() {
  sailing.value = true;
  clearTimeout(shipTimer);
  // The animation's end clears it; this is for a browser that never runs it.
  shipTimer = setTimeout(() => {
    sailing.value = false;
  }, 4000);
}

onUnmounted(() => {
  clearTimeout(statusTimer);
  clearTimeout(shipTimer);
  clearTimeout(dropTimer);
});

// The new page is revealed by a circle growing from the anchor. Browsers
// without view transitions, and visitors who ask for less motion, get the
// flip at once, with no ship. Nothing waits on a timer: the state flips
// inside the transition's callback.
const anchor = ref<HTMLElement>();
let busy = false;

async function toggle() {
  if (busy) return;
  const next = !props.modelValue;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced || typeof document.startViewTransition !== "function") {
    emit("update:modelValue", next);
    announce(next);
    if (next) drop();
    return;
  }

  busy = true;
  const root = document.documentElement;
  const box = anchor.value?.getBoundingClientRect();
  const x = box ? box.left + box.width / 2 : innerWidth / 2;
  const y = box ? box.top + box.height / 2 : 0;
  const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
  let flipped = false;
  const flip = () => {
    if (flipped) return;
    flipped = true;
    emit("update:modelValue", next);
  };
  root.classList.add(TURNING_CLASS);
  try {
    const transition = document.startViewTransition(async () => {
      flip();
      await nextTick();
    });
    await transition.ready;
    root.animate(
      { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
      { duration: 850, easing: "cubic-bezier(0.4, 0, 0.2, 1)", pseudoElement: "::view-transition-new(root)" },
    );
    await transition.finished;
  } catch {
    // A skipped transition (a hidden tab) still ran its callback; make sure.
    flip();
  } finally {
    root.classList.remove(TURNING_CLASS);
    busy = false;
  }
  announce(next);
  // The page is frozen while a transition runs, so these join after it.
  if (next) {
    drop();
    sail();
  }
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    class="hk-shanty-chip"
    :class="{ 'is-on': modelValue, 'is-pending': !hydrated, 'is-dropping': dropping }"
    @click="toggle"
  >
    <span ref="anchor" class="hk-shanty-chip-anchor" aria-hidden="true">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="12" cy="4.5" r="2" />
        <path
          d="M12 6.5V21M8.5 10h7M5 14.5c.6 3.6 3.4 6.5 7 6.5s6.4-2.9 7-6.5M3.6 16 5 14.5 6.6 16M17.4 16 19 14.5 20.4 16"
        />
      </svg>
    </span>
    <span><strong>Yo ho!</strong> Sea shanty mode</span>
    <span class="hk-shanty-chip-track" aria-hidden="true"><span class="hk-shanty-chip-knob"></span></span>
  </button>
  <p class="visually-hidden" role="status">{{ status }}</p>
  <div v-if="sailing" class="hk-ship" aria-hidden="true" @animationend.self="sailing = false">
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
/* Mise's announcement chip, as a switch: same pill, same disc, and the
   choice shown on a track at its end. */
.hk-shanty-chip {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  width: fit-content;
  max-width: 100%;
  min-height: 36px;
  margin-bottom: 24px;
  padding: 5px 14px 5px 5px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 999px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 1.4;
  text-align: left;
  cursor: pointer;
  transition:
    background-color 0.15s,
    border-color 0.15s,
    color 0.15s;
}
/* Stretches the target to 44 px without stretching the pill. */
.hk-shanty-chip::after {
  content: "";
  position: absolute;
  inset: -4px;
}
.hk-shanty-chip strong {
  color: var(--vp-c-text-1);
  font-weight: 600;
}
.hk-shanty-chip:hover {
  color: var(--vp-c-text-1);
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-soft);
}
/* The default theme's button focus ring is more specific than the page's. */
.hk-shanty-chip:focus-visible {
  outline: 2px solid var(--vp-c-brand-1);
  outline-offset: 4px;
}
.hk-shanty-chip.is-pending {
  visibility: hidden;
}
.hk-shanty-chip-anchor {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  overflow: hidden;
  border-radius: 50%;
  color: var(--vp-button-brand-text);
  background: var(--vp-button-brand-bg);
}
.hk-shanty-chip-anchor svg {
  width: 16px;
  height: 16px;
  transform-origin: 50% 25%;
}
/* Each plays once. At rest the anchor is still, so reduced motion (which
   removes animations) shows the finished state. The drop comes last: it ties
   the sway on specificity and must win while both apply. */
.hk-shanty-chip:hover .hk-shanty-chip-anchor svg,
.hk-shanty-chip:focus-visible .hk-shanty-chip-anchor svg {
  animation: hk-anchor-sway 0.9s ease-in-out;
}
.hk-shanty-chip.is-dropping .hk-shanty-chip-anchor svg {
  animation: hk-anchor-drop 0.7s cubic-bezier(0.3, 1.4, 0.5, 1);
}
@keyframes hk-anchor-drop {
  from {
    transform: translateY(-16px) rotate(-12deg);
  }
  60% {
    transform: translateY(2px) rotate(4deg);
  }
  to {
    transform: none;
  }
}
@keyframes hk-anchor-sway {
  25% {
    transform: rotate(-14deg);
  }
  75% {
    transform: rotate(14deg);
  }
}
.hk-shanty-chip-track {
  position: relative;
  flex: none;
  width: 30px;
  height: 16px;
  border: 1.5px solid var(--vp-c-text-2);
  border-radius: 999px;
  transition:
    background-color 0.15s,
    border-color 0.15s;
}
.hk-shanty-chip-knob {
  position: absolute;
  top: 50%;
  left: 2px;
  width: 9px;
  height: 9px;
  margin-top: -4.5px;
  border-radius: 50%;
  background: var(--vp-c-text-2);
  transition:
    transform 0.15s,
    background-color 0.15s;
}
.hk-shanty-chip.is-on .hk-shanty-chip-track {
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-1);
}
.hk-shanty-chip.is-on .hk-shanty-chip-knob {
  background: var(--vp-button-brand-text);
  transform: translateX(14px);
}
@media (forced-colors: active) {
  .hk-shanty-chip-track {
    border-color: CanvasText;
  }
  .hk-shanty-chip-knob {
    background: CanvasText;
  }
  .hk-shanty-chip.is-on .hk-shanty-chip-track {
    border-color: Highlight;
    background: Highlight;
  }
  .hk-shanty-chip.is-on .hk-shanty-chip-knob {
    background: HighlightText;
  }
}
/* A pill this narrow wraps its label, and a full round would clip it. */
@media (max-width: 359px) {
  .hk-shanty-chip {
    border-radius: 18px;
  }
}

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
