<template>
  <div class="home-page">
    <section class="hero-section">
      <div class="hero-background">
        <div class="grid-pattern"></div>
        <div class="floating-shapes">
          <div class="shape shape-1"></div>
          <div class="shape shape-2"></div>
          <div class="shape shape-3"></div>
        </div>
      </div>

      <div class="hero-content">
        <!-- Logo removed - clean design without SVG logo -->

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
            <button class="copy-btn" @click="copyToClipboard">
              <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M8 5H6C5.46957 5 4.96086 5.21071 4.58579 5.58579C4.21071 5.96086 4 6.46957 4 7V19C4 19.5304 4.21071 20.0391 4.58579 20.4142C4.96086 20.7893 5.46957 21 6 21H16C16.5304 21 17.0391 20.7893 17.4142 20.4142C17.7893 20.0391 18 19.5304 18 19V18M8 5C8 5.53043 8.21071 6.03914 8.58579 6.41421C8.96086 6.78929 9.46957 7 10 7H12C12.5304 7 13.0391 6.78929 13.4142 6.41421C13.7893 6.03914 14 5.53043 14 5M8 5C8 4.46957 8.21071 3.96086 8.58579 3.58579C8.96086 3.21071 9.46957 3 10 3H12C12.5304 3 13.0391 3.21071 13.4142 3.58579C13.7893 3.96086 14 4.46957 14 5M14 5H16C16.5304 5 17.0391 5.21071 17.4142 5.58579C17.7893 5.96086 18 6.46957 18 7V10" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
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
          <h3>Lightning Fast</h3>
          <p>Built with Rust, runs hook steps in parallel for maximum speed</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M17 21v-8H7v8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M7 3v5h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Smart Stashing</h3>
          <p>Fixes staged changes without touching unstaged hunks, preserving your workflow</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">
            <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M9 11L12 14L22 4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
              <path d="M21 12V19C21 19.5304 20.7893 20.0391 20.4142 20.4142C20.0391 20.7893 19.5304 21 19 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V5C3 4.46957 3.21071 3.96086 3.58579 3.58579C3.96086 3.21071 4.46957 3 5 3H16" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>
          <h3>Built-in Linters</h3>
          <p>Extensive collection of pre-configured linters and formatters</p>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const copyToClipboard = () => {
  navigator.clipboard.writeText('mise use hk')
  // You could add a toast notification here
}
</script>

<style scoped>
.home-page {
  position: relative;
  z-index: 1;
}

.hero-section {
  position: relative;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  contain: paint;
  z-index: 1;
}

.hero-background {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
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

.floating-shapes {
  position: absolute;
  width: 100%;
  height: 100%;
  overflow: hidden;
  contain: paint;
}

.shape {
  position: absolute;
  border-radius: 50%;
  opacity: 0.1;
  background: white;
  animation: float 20s infinite ease-in-out;
}

.shape-1 {
  width: 400px;
  height: 400px;
  top: 10%;
  left: 10%;
  animation-delay: 0s;
}

.shape-2 {
  width: 300px;
  height: 300px;
  top: 60%;
  right: 10%;
  animation-delay: 5s;
}

.shape-3 {
  width: 200px;
  height: 200px;
  bottom: 10%;
  left: 50%;
  animation-delay: 10s;
}

@keyframes float {
  0%, 100% {
    transform: translate(0, 0) scale(1);
  }
  33% {
    transform: translate(30px, -30px) scale(1.1);
  }
  66% {
    transform: translate(-30px, 30px) scale(0.9);
  }
}

.hero-content {
  position: relative;
  z-index: 1;
  text-align: center;
  padding: 2rem;
  max-width: 900px;
}

.hero-title {
  margin-bottom: 2rem;
  margin-top: 4rem;
}

.title-main {
  display: block;
  font-family: 'Bebas Neue', sans-serif;
  font-size: 6rem;
  font-weight: 400;
  color: white;
  text-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  letter-spacing: 6px;
  text-transform: uppercase;
  margin-bottom: 0;
  line-height: 1;
}

.title-sub {
  display: block;
  font-family: 'Bebas Neue', sans-serif;
  font-size: 2rem;
  font-weight: 400;
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: 4px;
  text-transform: uppercase;
  margin-top: 0.5rem;
  line-height: 1.2;
}

.hero-description {
  font-size: 1.2rem;
  color: rgba(255, 255, 255, 0.85);
  margin-bottom: 3rem;
  margin-top: 1.5rem;
  max-width: 600px;
  margin-left: auto;
  margin-right: auto;
  line-height: 1.6;
}

.hero-actions {
  display: flex;
  gap: 1rem;
  justify-content: center;
  margin-bottom: 3rem;
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
  color: #667eea;
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
  display: inline-block;
}

.code-snippet {
  position: relative;
  background: rgba(0, 0, 0, 0.3);
  backdrop-filter: blur(10px);
  padding: 1rem 4rem 1rem 1.5rem;
  border-radius: 12px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  min-width: 280px;
}

.code-label {
  font-family: 'Bebas Neue', sans-serif;
  font-size: 0.9rem;
  color: rgba(255, 255, 255, 0.7);
  font-weight: 400;
  text-transform: uppercase;
  letter-spacing: 1.5px;
  white-space: nowrap;
}

.code-snippet code {
  font-family: 'JetBrains Mono', monospace;
  font-size: 1.1rem;
  color: white;
  font-weight: 600;
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
  width: 18px;
  height: 18px;
  stroke: white;
}

.features-section {
  padding: 3.5rem 2rem 6rem;
  max-width: 1200px;
  margin: 0 auto;
  position: relative;
  background: var(--vp-c-bg);
  min-height: 100vh;
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
  background: linear-gradient(135deg, #667eea, #764ba2);
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
  background: linear-gradient(90deg, #667eea, #764ba2);
  transform: scaleX(0);
  transform-origin: left;
  transition: transform 0.3s ease;
}

.feature-card:hover {
  transform: translateY(-5px);
  border-color: rgba(102, 126, 234, 0.3);
  box-shadow: 0 10px 30px rgba(102, 126, 234, 0.15);
}

.feature-card:hover::before {
  transform: scaleX(1);
}

.feature-icon {
  display: inline-flex;
  padding: 1rem;
  background: linear-gradient(135deg, rgba(102, 126, 234, 0.1), rgba(118, 75, 162, 0.1));
  border-radius: 12px;
  margin-bottom: 1rem;
}

.feature-icon svg {
  width: 32px;
  height: 32px;
  stroke: #667eea;
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
  .title-main {
    font-size: 3.5rem;
    letter-spacing: 3px;
  }

  .title-sub {
    font-size: 1.3rem;
  }

  .hero-description {
    font-size: 1.1rem;
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