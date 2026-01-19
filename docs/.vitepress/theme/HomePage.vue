<template>
  <div class="home-page">
    <section class="hero-section">
      <div class="hero-background">
        <div class="grid-pattern"></div>
      </div>

      <div class="hero-content">
        <div class="hero-main">
          <img src="/logo.png" alt="hk logo" class="hero-logo" />

          <div class="hero-text">
            <h1 class="hero-title">
              <span class="title-main">hk</span>
              <span class="title-sub">Git Hook Manager</span>
            </h1>

            <p class="hero-description">
              Fast, powerful, and flexible hook management for modern development workflows
            </p>

            <div class="hero-actions">
              <a href="/getting_started" class="btn btn-primary">
                <span class="btn-text">Get Started</span>
                <svg class="btn-icon" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <path d="M5 12H19M19 12L12 5M19 12L12 19" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </a>
              <a href="/configuration" class="btn btn-secondary">
                <span class="btn-text">Configuration</span>
              </a>
            </div>

            <div class="quick-start">
              <div class="code-snippet">
                <span class="code-label">Quick Install</span>
                <code>mise use hk</code>
                <button class="copy-btn" :class="{ copied }" @click="copyToClipboard">
                  <svg v-if="!copied" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M8 5H6C5.46957 5 4.96086 5.21071 4.58579 5.58579C4.21071 5.96086 4 6.46957 4 7V19C4 19.5304 4.21071 20.0391 4.58579 20.4142C4.96086 20.7893 5.46957 21 6 21H16C16.5304 21 17.0391 20.7893 17.4142 20.4142C17.7893 20.0391 18 19.5304 18 19V18M8 5C8 5.53043 8.21071 6.03914 8.58579 6.41421C8.96086 6.78929 9.46957 7 10 7H12C12.5304 7 13.0391 6.78929 13.4142 6.41421C13.7893 6.03914 14 5.53043 14 5M8 5C8 4.46957 8.21071 3.96086 8.58579 3.58579C8.96086 3.21071 9.46957 3 10 3H12C12.5304 3 13.0391 3.21071 13.4142 3.58579C13.7893 3.96086 14 4.46957 14 5M14 5H16C16.5304 5 17.0391 5.21071 17.4142 5.58579C17.7893 5.96086 18 6.46957 18 7V10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M20 6L9 17L4 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="features-section">
      <h2 class="section-title">Why Choose HK?</h2>

      <div class="features-grid">
        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M13 2L3 14H12L11 22L21 10H12L13 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Shipshape</h3>
          <p>Clean, efficient, and built to perform</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M17 21v-8H7v8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M7 3v5h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Batten Down the Hatches</h3>
          <p>Secures staged changes while keeping unstaged work safe in port</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M9 11L12 14L22 4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M21 12V19C21 19.5304 20.7893 20.0391 20.4142 20.4142C20.0391 20.7893 19.5304 21 19 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V5C3 4.46957 3.21071 3.96086 3.58579 3.58579C3.96086 3.21071 4.46957 3 5 3H16" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Tackle Box</h3>
          <p>A well-stocked collection of pre-rigged linters and formatters</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M12 18V12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M9 15L12 12L15 15" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Captain's Log</h3>
          <p>Type-safe, readable configuration that charts your course</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M12 2L2 7L12 12L22 7L12 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M2 17L12 22L22 17" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M2 12L12 17L22 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>First Mate</h3>
          <p>Your trusted crew member handling tool provisioning</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M17 21V19C17 17.9391 16.5786 16.9217 15.8284 16.1716C15.0783 15.4214 14.0609 15 13 15H5C3.93913 15 2.92172 15.4214 2.17157 16.1716C1.42143 16.9217 1 17.9391 1 19V21" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M9 11C11.2091 11 13 9.20914 13 7C13 4.79086 11.2091 3 9 3C6.79086 3 5 4.79086 5 7C5 9.20914 6.79086 11 9 11Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M23 21V19C22.9993 18.1137 22.7044 17.2528 22.1614 16.5523C21.6184 15.8519 20.8581 15.3516 20 15.13" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M16 3.13C16.8604 3.35031 17.623 3.85071 18.1676 4.55232C18.7122 5.25392 19.0078 6.11683 19.0078 7.005C19.0078 7.89318 18.7122 8.75608 18.1676 9.45769C17.623 10.1593 16.8604 10.6597 16 10.88" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>All Hands on Deck</h3>
          <p>Smart file locking lets your whole crew work simultaneously</p>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const copied = ref(false)

const copyToClipboard = async () => {
  try {
    await navigator.clipboard.writeText('mise use hk')
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    // Clipboard write failed - don't show success state
  }
}
</script>

<style scoped>
.home-page {
  position: relative;
  z-index: 1;
}

.hero-section {
  position: relative;
  min-height: auto;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  contain: paint;
  z-index: 1;
  padding: 3rem 0 2rem;
}

.hero-background {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background: linear-gradient(135deg, #0a1628 0%, #0f3460 50%, #16213e 100%);
  pointer-events: none;
}

.grid-pattern {
  position: absolute;
  width: 100%;
  height: 100%;
  opacity: 0.03;
  background-image:
    linear-gradient(rgba(255, 255, 255, 0.2) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.2) 1px, transparent 1px);
  background-size: 50px 50px;
  animation: grid-move 20s linear infinite;
}

@keyframes grid-move {
  0% {
    transform: translate(0, 0);
  }
  100% {
    transform: translate(50px, 50px);
  }
}

.hero-content {
  position: relative;
  z-index: 1;
  text-align: center;
  padding: 2rem;
  max-width: 1000px;
}

.hero-main {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 2.5rem;
  margin-bottom: 1.5rem;
}

.hero-logo {
  max-width: 180px;
  height: auto;
  filter: drop-shadow(0 8px 30px rgba(76, 201, 240, 0.4)) drop-shadow(0 4px 15px rgba(0, 0, 0, 0.3));
  flex-shrink: 0;
}

.hero-text {
  text-align: left;
}

.hero-title {
  margin-bottom: 0.5rem;
  margin-top: 0;
}

.title-main {
  display: block;
  font-family: 'Bebas Neue', sans-serif;
  font-size: 5.5rem;
  font-weight: 400;
  color: white;
  text-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  letter-spacing: 6px;
  text-transform: uppercase;
  margin-bottom: 0;
  line-height: 0.85;
}

.title-sub {
  display: block;
  font-family: 'Bebas Neue', sans-serif;
  font-size: 1.8rem;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: 4px;
  text-transform: uppercase;
  margin-top: 0.25rem;
  line-height: 1.2;
}

.hero-description {
  font-size: 1.05rem;
  color: rgba(255, 255, 255, 0.85);
  margin-bottom: 0;
  margin-top: 0.75rem;
  max-width: 320px;
  line-height: 1.6;
}

.hero-actions {
  display: flex;
  gap: 1rem;
  justify-content: flex-start;
  margin-top: 1.25rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  padding: 1rem 2rem;
  border-radius: 50px;
  font-family: 'Bebas Neue', sans-serif;
  font-weight: 400;
  font-size: 1.3rem;
  letter-spacing: 2px;
  text-transform: uppercase;
  text-decoration: none;
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
}

.btn-primary {
  background: white;
  color: #0077a0;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
}

.btn-primary:hover {
  transform: translateY(-3px);
  box-shadow: 0 15px 40px rgba(0, 0, 0, 0.3);
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.1);
  color: white;
  border: 2px solid rgba(255, 255, 255, 0.3);
  backdrop-filter: blur(10px);
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.2);
  border-color: rgba(255, 255, 255, 0.5);
  transform: translateY(-2px);
}

.btn-icon {
  width: 20px;
  height: 20px;
  transition: transform 0.3s ease;
}

.btn:hover .btn-icon {
  transform: translateX(5px);
}

.quick-start {
  display: block;
}

.code-snippet {
  position: relative;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(10px);
  padding: 0.6rem 3.5rem 0.6rem 1rem;
  border-radius: 10px;
  display: inline-flex;
  align-items: baseline;
  gap: 0.75rem;
}

.code-label {
  font-family: 'Inter', sans-serif;
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.5);
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 1px;
  white-space: nowrap;
}

.code-snippet code {
  font-family: 'JetBrains Mono', monospace;
  font-size: 1rem;
  color: white;
  font-weight: 500;
  letter-spacing: 0;
  white-space: nowrap;
}

.copy-btn {
  position: absolute;
  right: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  padding: 0.5rem;
  cursor: pointer;
  transition: all 0.3s ease;
}

.copy-btn:hover {
  background: rgba(255, 255, 255, 0.2);
  border-color: rgba(255, 255, 255, 0.4);
}

.copy-btn svg {
  width: 16px;
  height: 16px;
  stroke: white;
  transition: stroke 0.2s ease;
}

.copy-btn.copied {
  background: rgba(76, 201, 140, 0.3);
  border-color: rgba(76, 201, 140, 0.5);
}

.copy-btn.copied svg {
  stroke: #4CC98C;
}

.features-section {
  padding: 2rem 2rem 4rem;
  max-width: 1200px;
  margin: 0 auto;
  position: relative;
  background: var(--vp-c-bg);
}

.section-title {
  text-align: center;
  font-family: 'Bebas Neue', sans-serif;
  font-size: 3rem;
  font-weight: 400;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  margin-bottom: 4rem;
  padding-top: 1rem;
  line-height: 1.3;
  background: linear-gradient(135deg, #4CC9F0, #00B4D8);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  display: block;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 2rem;
}

.feature-card {
  background: var(--vp-c-bg-soft);
  padding: 2rem;
  border-radius: 16px;
  text-align: center;
  transition: all 0.3s ease;
  border: 1px solid transparent;
  position: relative;
  overflow: hidden;
}

.feature-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 4px;
  background: linear-gradient(90deg, #4CC9F0, #00B4D8);
  transform: scaleX(0);
  transform-origin: left;
  transition: transform 0.3s ease;
}

.feature-card:hover {
  transform: translateY(-5px);
  border-color: rgba(76, 201, 240, 0.3);
  box-shadow: 0 10px 30px rgba(76, 201, 240, 0.15);
}

.feature-card:hover::before {
  transform: scaleX(1);
}

.feature-icon {
  display: inline-flex;
  padding: 1rem;
  background: linear-gradient(135deg, rgba(76, 201, 240, 0.1), rgba(0, 180, 216, 0.1));
  border-radius: 12px;
  margin-bottom: 1rem;
}

.feature-icon svg {
  width: 32px;
  height: 32px;
  stroke: #4CC9F0;
}

.feature-card h3 {
  font-family: 'Bebas Neue', sans-serif;
  font-size: 1.6rem;
  font-weight: 400;
  letter-spacing: 1px;
  text-transform: uppercase;
  margin-bottom: 1rem;
  color: var(--vp-c-text-1);
  line-height: 1.3;
}

.feature-card p {
  color: var(--vp-c-text-2);
  line-height: 1.6;
}

@media (max-width: 768px) {
  .hero-main {
    flex-direction: column;
    gap: 1.5rem;
  }

  .hero-text {
    text-align: center;
  }

  .hero-description {
    max-width: 100%;
  }

  .title-main {
    font-size: 3.5rem;
    letter-spacing: 3px;
  }

  .title-sub {
    font-size: 1.3rem;
  }

  .hero-actions {
    flex-direction: column;
    align-items: center;
  }

  .btn {
    width: 100%;
    max-width: 300px;
    justify-content: center;
  }
}
</style>
