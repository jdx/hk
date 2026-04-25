import { h, onMounted } from 'vue'
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
    onMounted(() => {
      const addStarCount = () => {
        const githubLink = document.querySelector(
          '.VPSocialLinks a[href*="github.com/jdx/hk"]',
        )
        if (githubLink && !githubLink.querySelector('.star-count')) {
          const starBadge = document.createElement('span')
          starBadge.className = 'star-count'
          starBadge.textContent = starsData.stars
          starBadge.title = 'GitHub Stars'
          githubLink.appendChild(starBadge)
        }
      }

      addStarCount()
      setTimeout(addStarCount, 100)
      const observer = new MutationObserver(addStarCount)
      observer.observe(document.body, { childList: true, subtree: true })
    })
  },
} satisfies Theme
