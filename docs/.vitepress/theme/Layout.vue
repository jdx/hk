<script setup lang="ts">
import DefaultTheme from "vitepress/theme";
import EndevFooter from "./EndevFooter.vue";
import EndevSponsors from "./EndevSponsors.vue";
import ShantyNotice from "./ShantyNotice.vue";
import ShantyOverlay from "./ShantyOverlay.vue";
import ShantySwitch from "./ShantySwitch.vue";

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
      <EndevSponsors />
      <EndevFooter />
      <ShantyOverlay />
    </template>
  </Layout>
</template>
