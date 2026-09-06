<script setup lang="ts">
import DefaultTheme from "vitepress/theme";
import EndevFooter from "./EndevFooter.vue";
import EndevSponsors from "./EndevSponsors.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #layout-bottom>
      <EndevSponsors />
      <EndevFooter />
    </template>
  </Layout>
</template>
