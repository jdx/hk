<script setup lang="ts">
import { useData, useRoute } from "vitepress";
import { computed } from "vue";
import { englishPath, isPiratePath } from "./shanty-mode";
import { useShantyMode } from "./useShantyMode";

// A note above a page in sea shanty mode that is not the whole story: a pirate
// page written from an older English page (the build marks it, see
// pirate-pages.mjs), or an English page shown in the mode because it has no
// pirate variant yet.
const { frontmatter } = useData();
const route = useRoute();
const { on, readEnglish } = useShantyMode();

const pirate = computed(() => isPiratePath(route.path));
const stale = computed(() => pirate.value && frontmatter.value.pirateStale === true);
const untranslated = computed(() => !pirate.value && on.value);
const english = computed(() => englishPath(route.path));
</script>

<template>
  <aside v-if="stale" class="hk-shanty-notice" aria-label="About this page">
    <p>
      <strong>Arr, this chart be behind the times.</strong> The English page was
      redrawn after this one was inked, so some o' what follows may be out o'
      date.
      <a :href="english" @click.prevent="readEnglish">Read the latest English page →</a>
    </p>
  </aside>
  <aside v-else-if="untranslated" class="hk-shanty-notice" aria-label="About this page">
    <p>
      <strong>No pirate hand has inked this page yet,</strong> so here it be in
      plain English.
    </p>
  </aside>
</template>

<style scoped>
.hk-shanty-notice {
  margin: 0 0 24px;
  padding: 12px 16px;
  border: 1px solid var(--hk-sea-brass, var(--vp-c-divider));
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-2);
  font-size: 14px;
  line-height: 1.6;
}
.hk-shanty-notice p {
  margin: 0;
}
.hk-shanty-notice strong {
  color: var(--vp-c-text-1);
}
.hk-shanty-notice a {
  color: var(--vp-c-brand-1);
  font-weight: 500;
  white-space: nowrap;
}
</style>
