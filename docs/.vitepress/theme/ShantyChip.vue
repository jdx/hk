<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { useShantyMode } from "./useShantyMode";

// The landing page's switch for sea shanty mode, in the place mise puts its
// announcement chip: above the title. It flips the same mode as the switch in
// the header (useShantyMode.ts): on, the landing page becomes its pirate
// variant.
const { on, toggle, sailing } = useShantyMode();

// The server-rendered page has no switch to press: it is space kept open, and
// it appears once the page can act on a press, so nothing dead is on screen
// and nothing moves when it arrives.
const hydrated = ref(false);
onMounted(() => {
  hydrated.value = true;
});

// The anchor drops into its disc once, when the mode turns on: the pirate
// landing page is a new page, so this chip is new too, and it takes its cue
// from the ship setting sail. It is a state of its own, not a look of
// `.is-on`: that would replay whenever hover or focus left the chip, and lose
// to the hover sway at the moment of switching on.
const dropping = ref(false);
let dropTimer: ReturnType<typeof setTimeout> | undefined;
watch(sailing, (now) => {
  if (!now) return;
  dropping.value = true;
  clearTimeout(dropTimer);
  dropTimer = setTimeout(() => {
    dropping.value = false;
  }, 800);
});
onUnmounted(() => clearTimeout(dropTimer));

const anchor = ref<HTMLElement>();
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="on"
    class="hk-shanty-chip"
    :class="{ 'is-on': on, 'is-pending': !hydrated, 'is-dropping': dropping }"
    @click="toggle(anchor)"
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
</style>
