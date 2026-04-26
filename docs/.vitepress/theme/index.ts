import { h, onMounted, onUnmounted } from 'vue'
import type { Theme } from 'vitepress'
import DefaultTheme from 'vitepress/theme-without-fonts'
import Layout from './Layout.vue'
import HomePage from './HomePage.vue'
import { initBanner } from './banner'
import { data as starsData } from '../stars.data'
import './style.css'

export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app, router, siteData }) {
    app.component('HomePage', HomePage)
    initBanner()
  },
  setup() {
    let observer: MutationObserver | undefined
    onMounted(() => {
      const addStarCount = () => {
        if (!starsData.stars) return false

        const githubLinks = document.querySelectorAll(
          '.VPSocialLinks a[href*="github.com/jdx/hk"]',
        )
        githubLinks.forEach((githubLink) => {
          if (!githubLink.querySelector('.star-count')) {
            const starBadge = document.createElement('span')
            starBadge.className = 'star-count'
            starBadge.textContent = `★ ${starsData.stars}`
            starBadge.title = 'GitHub Stars'
            githubLink.appendChild(starBadge)
          }
        })
        return githubLinks.length > 0 && Array.from(githubLinks).every((link) => link.querySelector('.star-count'))
      }

      if (addStarCount()) return

      observer = new MutationObserver(() => {
        if (addStarCount()) observer?.disconnect()
      })
      observer.observe(document.querySelector('.VPNav') || document.body, { childList: true, subtree: true })
    })
    onUnmounted(() => observer?.disconnect())
  },
} satisfies Theme
