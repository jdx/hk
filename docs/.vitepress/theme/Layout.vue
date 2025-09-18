<template>
  <Layout>
    <!-- Custom header banner removed - using HomePage component instead -->

    <!-- Removed nav-bar-title-after to avoid duplicate "hk" -->

    <!-- Removed footer decoration to prevent visual issues -->
  </Layout>
</template>

<script setup lang="ts">
import DefaultTheme from 'vitepress/theme'
import { useData } from 'vitepress'

const { Layout } = DefaultTheme
const { page } = useData()
</script>

<style scoped>
.custom-header-banner {
  position: relative;
  margin: -32px -24px 32px;
  padding: 48px 24px;
  overflow: hidden;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.animated-bg {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  opacity: 0.1;
  background-image:
    repeating-linear-gradient(45deg, transparent, transparent 35px, rgba(255, 255, 255, 0.5) 35px, rgba(255, 255, 255, 0.5) 70px);
  animation: slide 20s linear infinite;
}

@keyframes slide {
  from {
    transform: translateX(0);
  }
  to {
    transform: translateX(70px);
  }
}

.header-content {
  position: relative;
  z-index: 1;
  text-align: center;
  max-width: 800px;
  margin: 0 auto;
}

.hero-title {
  font-family: 'Bebas Neue', sans-serif;
  font-size: 4rem;
  margin-bottom: 1rem;
  color: white;
  text-shadow: 2px 4px 8px rgba(0, 0, 0, 0.1);
  text-transform: uppercase;
  letter-spacing: 3px;
}

.brand-hk {
  font-weight: 400;
  background: linear-gradient(45deg, #fff, #e0e7ff);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  display: inline-block;
  margin-right: 1rem;
}

.tagline {
  font-size: 1.5rem;
  font-weight: 300;
  opacity: 0.95;
  letter-spacing: 2px;
}

.hero-description {
  font-size: 1.2rem;
  color: rgba(255, 255, 255, 0.9);
  max-width: 600px;
  margin: 0 auto;
}

/* Removed nav-brand styles - no longer used */

.custom-footer-section {
  margin-top: 48px;
  padding-top: 24px;
  border-top: 1px solid var(--vp-c-divider);
}

.footer-decoration {
  height: 4px;
  background: linear-gradient(90deg, #667eea, #764ba2, #667eea);
  background-size: 200% 100%;
  animation: gradient 3s ease infinite;
  border-radius: 2px;
}

@keyframes gradient {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

@media (max-width: 768px) {
  .hero-title {
    font-size: 2.5rem;
  }

  .tagline {
    font-size: 1.2rem;
  }

  .hero-description {
    font-size: 1rem;
  }
}
</style>