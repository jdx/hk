<script setup lang="ts">
// The theme without VitePress's Inter: style.css sets the site's own fonts,
// so Inter was never shown, only preloaded and downloaded.
import DefaultTheme from "vitepress/theme-without-fonts";
import EndevFooter from "./EndevFooter.vue";
import EndevSponsors from "./EndevSponsors.vue";
import ShantyNotice from "./ShantyNotice.vue";
import ShantyOverlay from "./ShantyOverlay.vue";
import ShantySwitch from "./ShantySwitch.vue";
import ShantyWaves from "./ShantyWaves.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #nav-bar-content-after>
      <ShantySwitch />
    </template>
    <template #nav-screen-content-after>
      <ShantySwitch screen />
    </template>
    <template #doc-before>
      <ShantyNotice />
    </template>
    <template #layout-bottom>
      <ShantyWaves />
      <EndevSponsors />
      <EndevFooter />
      <ShantyOverlay />
    </template>
  </Layout>
</template>

<style>
/* The small-screen menu's slot for the switch's row comes after the social
   links; the row belongs under the appearance row it is drawn like, so the
   menu is a column and the links go after it. */
.VPNavScreen .container {
  display: flex;
  flex-direction: column;
}
.VPNavScreen .container > .social-links {
  order: 1;
}
.VPNavScreen .container::after {
  order: 2;
}
</style>
