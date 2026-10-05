<script setup lang="ts">
// The swell at the foot of every page in sea shanty mode: three rows of
// engraved waves, drifting. It is always in the page and shown only by the
// mode's class, which is on <html> before the first paint, so turning up on
// a themed page moves nothing after it loads. The rows move by transform
// alone, so the drift costs no repaint; with reduced motion they stay still.
</script>

<template>
  <div class="hk-waves" aria-hidden="true">
    <div class="hk-wave hk-wave-far" />
    <div class="hk-wave hk-wave-mid" />
    <div class="hk-wave hk-wave-near" />
  </div>
</template>

<style scoped>
.hk-waves {
  display: none;
}
:root.shanty-mode .hk-waves {
  position: relative;
  display: block;
  height: 64px;
  margin-top: 32px;
  overflow: hidden;
  pointer-events: none;
}
.hk-wave {
  position: absolute;
  left: 0;
  width: calc(100% + 48px);
  height: 12px;
  -webkit-mask: var(--hk-art-wave) left center / 48px 12px repeat-x;
  mask: var(--hk-art-wave) left center / 48px 12px repeat-x;
  animation: hk-drift 24s linear infinite;
}
.hk-wave-far {
  top: 10px;
  background-color: var(--hk-sea-brass);
  opacity: 0.25;
  animation-duration: 38s;
  animation-direction: reverse;
}
.hk-wave-mid {
  top: 26px;
  left: -16px;
  background-color: var(--vp-c-brand-1);
  opacity: 0.35;
  animation-duration: 29s;
}
.hk-wave-near {
  top: 42px;
  left: -32px;
  background-color: var(--hk-sea-brass);
  opacity: 0.55;
  animation-duration: 21s;
}
@keyframes hk-drift {
  to {
    transform: translateX(-48px);
  }
}
@media (forced-colors: active) {
  :root.shanty-mode .hk-waves {
    display: none;
  }
}
</style>
