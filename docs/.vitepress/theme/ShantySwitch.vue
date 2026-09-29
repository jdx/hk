<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useShantyMode } from "./useShantyMode";

// Sea shanty mode's switch in the header, on every page, drawn like the
// appearance switch beside it: a track with an anchor on its knob. `screen`
// is the row in the small-screen menu, with its label beside the switch.

defineProps<{ screen?: boolean }>();
const { on, toggle } = useShantyMode();

// The server-rendered page has no switch to press: it is space kept open, and
// it appears once the page can act on a press.
const hydrated = ref(false);
onMounted(() => {
  hydrated.value = true;
});
const knob = ref<HTMLElement>();
</script>

<template>
  <div class="hk-shanty-switch" :class="{ 'is-screen': screen, 'is-pending': !hydrated }">
    <span v-if="screen" class="hk-shanty-switch-label" aria-hidden="true">Sea shanty mode</span>
    <button
      type="button"
      role="switch"
      class="hk-shanty-switch-button"
      :class="{ 'is-on': on }"
      :aria-checked="on"
      aria-label="Sea shanty mode"
      :title="on ? 'Sea shanty mode is on: back to plain English' : 'Sea shanty mode: the docs as the crew sings them'"
      @click="toggle(knob)"
    >
      <span ref="knob" class="hk-shanty-switch-knob">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <circle cx="12" cy="4.5" r="2" />
          <path
            d="M12 6.5V21M8.5 10h7M5 14.5c.6 3.6 3.4 6.5 7 6.5s6.4-2.9 7-6.5M3.6 16 5 14.5 6.6 16M17.4 16 19 14.5 20.4 16"
          />
        </svg>
      </span>
    </button>
  </div>
</template>

<style scoped>
.hk-shanty-switch {
  display: flex;
  align-items: center;
}
.hk-shanty-switch.is-pending {
  visibility: hidden;
}
/* In the bar: after the social links, with the divider the bar puts
   between its groups. */
.hk-shanty-switch:not(.is-screen) {
  position: relative;
  margin-left: 16px;
  padding-left: 16px;
}
.hk-shanty-switch:not(.is-screen)::before {
  content: "";
  position: absolute;
  top: 50%;
  left: 0;
  width: 1px;
  height: 24px;
  margin-top: -12px;
  background-color: var(--vp-c-divider);
}
@media (max-width: 767px) {
  .hk-shanty-switch:not(.is-screen) {
    margin-left: 8px;
    padding-left: 0;
  }
  .hk-shanty-switch:not(.is-screen)::before {
    display: none;
  }
}
/* In the small-screen menu: a row like the appearance row above it. */
.hk-shanty-switch.is-screen {
  justify-content: space-between;
  margin-top: 8px;
  padding: 12px 14px 12px 16px;
  border-radius: 8px;
  background-color: var(--vp-c-bg-soft);
}
.hk-shanty-switch-label {
  color: var(--vp-c-text-2);
  font-size: 12px;
  font-weight: 500;
  line-height: 24px;
}

.hk-shanty-switch-button {
  position: relative;
  display: block;
  flex-shrink: 0;
  width: 40px;
  height: 22px;
  border: 1px solid var(--vp-input-border-color);
  border-radius: 11px;
  background-color: var(--vp-input-switch-bg-color);
  transition: border-color 0.25s, background-color 0.25s;
}
/* Stretches the target to 44 px without stretching the track. */
.hk-shanty-switch-button::after {
  content: "";
  position: absolute;
  inset: -11px -2px;
}
.hk-shanty-switch-button:hover {
  border-color: var(--vp-c-brand-1);
}
.hk-shanty-switch-knob {
  position: absolute;
  top: 1px;
  left: 1px;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  color: var(--vp-c-text-2);
  background-color: var(--vp-c-neutral-inverse);
  box-shadow: var(--vp-shadow-1);
  transition: transform 0.25s, color 0.25s, background-color 0.25s;
}
.hk-shanty-switch-knob svg {
  width: 12px;
  height: 12px;
  transform-origin: 50% 25%;
}
.hk-shanty-switch-button:hover .hk-shanty-switch-knob svg,
.hk-shanty-switch-button:focus-visible .hk-shanty-switch-knob svg {
  animation: hk-anchor-sway 0.9s ease-in-out;
}
.hk-shanty-switch-button.is-on {
  border-color: var(--hk-sea-brass, var(--vp-c-brand-1));
  background-color: var(--hk-sea-brass, var(--vp-c-brand-1));
}
.hk-shanty-switch-button.is-on .hk-shanty-switch-knob {
  color: var(--hk-sea-brass, var(--vp-c-brand-1));
  background-color: var(--vp-c-bg);
  transform: translateX(18px);
}
@keyframes hk-anchor-sway {
  25% {
    transform: rotate(-14deg);
  }
  75% {
    transform: rotate(14deg);
  }
}
@media (forced-colors: active) {
  .hk-shanty-switch-button,
  .hk-shanty-switch-button.is-on {
    border-color: CanvasText;
  }
  .hk-shanty-switch-button.is-on {
    background-color: Highlight;
  }
}
</style>
