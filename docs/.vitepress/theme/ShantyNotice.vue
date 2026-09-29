<script setup lang="ts">
import { useData, useRoute } from "vitepress";
import { computed } from "vue";
import { englishPath, isPiratePath } from "./shanty-mode";

// A note above a page in sea shanty mode that is not the whole story: a pirate
// page written from an older English page (the build marks it, see
// pirate-pages.mjs), or an English page shown in the mode because it has no
// pirate variant yet. The second is in every English page and shown only by
// the mode's class, which the pre-paint script sets: were it rendered once
// the saved choice is read, it would push the page down after it loads.
const { frontmatter } = useData();
const route = useRoute();

const pirate = computed(() => isPiratePath(route.path));
const stale = computed(() => pirate.value && frontmatter.value.pirateStale === true);
// `?shanty=0` makes this visit English (shanty-mode.ts), so the English page
// stays English, in this tab or a new one, until the switch is used.
const english = computed(() => `${englishPath(route.path)}?shanty=0`);
</script>

<template>
  <aside v-if="stale" class="hk-shanty-notice" aria-label="About this page">
    <p>
      <strong>Arr, this chart be behind the times.</strong> The English page was
      redrawn after this one was inked, so some o' what follows may be out o'
      date.
      <a :href="english">Read the latest English page →</a>
    </p>
  </aside>
  <aside v-else-if="!pirate" class="hk-shanty-notice is-untranslated" aria-label="About this page">
    <p>
      <strong>No pirate hand has inked this page yet,</strong> so here it be in
      plain English.
    </p>
  </aside>
</template>

<style scoped>
.hk-shanty-notice {
  display: flex;
  gap: 14px;
  align-items: center;
  margin: 0 0 32px;
  padding: 12px 18px 12px 14px;
  border: 1px solid var(--hk-sea-rule, var(--vp-c-divider));
  border-radius: 3px;
  background-color: var(--vp-c-bg-soft);
  background-image: var(--hk-grain);
  box-shadow: 0 12px 24px -18px var(--hk-sea-shade, rgb(0 0 0 / 30%));
  color: var(--vp-c-text-2);
  font-size: 15px;
  line-height: 1.6;
}
.hk-shanty-notice.is-untranslated {
  display: none;
}
:root.shanty-mode .hk-shanty-notice.is-untranslated {
  display: flex;
}
/* A message in a bottle. */
.hk-shanty-notice::before {
  content: "";
  flex-shrink: 0;
  width: 34px;
  height: 34px;
  background-color: var(--hk-sea-brass, currentColor);
  -webkit-mask: var(--hk-art-bottle) center / contain no-repeat;
  mask: var(--hk-art-bottle) center / contain no-repeat;
}
.hk-shanty-notice p {
  margin: 0;
  font-style: italic;
}
.hk-shanty-notice strong {
  color: var(--vp-c-text-1);
  font-weight: 600;
}
.hk-shanty-notice a {
  color: var(--vp-c-brand-1);
  font-style: normal;
  font-weight: 500;
  white-space: nowrap;
}
@media (forced-colors: active) {
  .hk-shanty-notice::before {
    display: none;
  }
}
</style>
