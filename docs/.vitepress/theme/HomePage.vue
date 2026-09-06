<script setup lang="ts">
import { onUnmounted, ref } from "vue";

const installCommand = "mise use hk";
const copyStatus = ref("");
let copyTimer: ReturnType<typeof setTimeout> | undefined;

async function copyInstall() {
  clearTimeout(copyTimer);
  try {
    await navigator.clipboard.writeText(installCommand);
    copyStatus.value = "Copied to clipboard";
  } catch {
    copyStatus.value = "Select the command to copy it manually";
  }
  copyTimer = setTimeout(() => {
    copyStatus.value = "";
  }, 4000);
}

onUnmounted(() => clearTimeout(copyTimer));
</script>

<template>
  <div class="hk-home">
    <section class="hk-hero" aria-labelledby="hero-title">
      <div class="hk-hero-copy">
        <p class="hk-eyebrow">
          <img src="/logo-small.png" width="32" height="32" alt="" /> Git hooks
          · linting · formatting
        </p>
        <h1 id="hero-title">
          Your checks.<br /><span>In parallel.</span><br />Before you commit.
        </h1>
        <p class="hk-intro">
          hk runs your linters and formatters together, coordinating access to
          each file so fixes don’t overwrite one another.
        </p>
        <div class="hk-actions">
          <a class="hk-button hk-button-primary" href="/getting_started"
            >Get started <span aria-hidden="true">→</span></a
          >
          <a class="hk-button" href="/reference/examples/"
            >Explore configurations</a
          >
        </div>
        <div class="hk-install">
          <span class="hk-prompt" aria-hidden="true">$</span>
          <code>{{ installCommand }}</code>
          <button
            type="button"
            aria-label="Copy mise install command"
            @click="copyInstall"
          >
            Copy
          </button>
        </div>
        <p class="hk-install-note" role="status">
          {{
            copyStatus ||
            "Install with mise, or choose another installation method."
          }}
          <a v-if="!copyStatus" href="/getting_started#installation"
            >All options →</a
          >
        </p>
      </div>

      <div class="hk-preview">
        <div class="hk-preview-header">
          <span>hk.pkl</span
          ><span class="hk-preview-label">One set of steps</span>
        </div>
        <pre
          class="hk-config"
          aria-label="Example steps shared by pre-commit and check hooks"
        ><code><span class="hk-code-keyword">local</span> linters = <span class="hk-code-keyword">new</span> Mapping&lt;String, Step&gt; {
  [<span class="hk-code-string">"prettier"</span>] = Builtins.prettier
  [<span class="hk-code-string">"eslint"</span>] = Builtins.eslint
  [<span class="hk-code-string">"ruff"</span>] = Builtins.ruff
}

hooks {
  [<span class="hk-code-string">"pre-commit"</span>] {
    fix = <span class="hk-code-keyword">true</span>
    stash = <span class="hk-code-string">"git"</span>
    steps = linters
  }
  [<span class="hk-code-string">"check"</span>] { steps = linters }
}</code></pre>
        <div class="hk-coordination">
          <p class="hk-eyebrow">Independent files, concurrent work</p>
          <div class="hk-lane">
            <code>app.ts</code>
            <div>
              <span class="hk-lane-check">check</span
              ><span class="hk-lane-fix">fix</span>
            </div>
          </div>
          <div class="hk-lane">
            <code>main.py</code>
            <div>
              <span class="hk-lane-check hk-lane-long">check</span
              ><span class="hk-lane-fix">fix</span>
            </div>
          </div>
          <p>Reads can overlap. Writes wait for access to the same file.</p>
        </div>
        <a
          class="hk-preview-link"
          href="/getting_started#your-first-configuration"
          >See the complete configuration <span aria-hidden="true">↗</span></a
        >
      </div>
    </section>

    <section class="hk-principles" aria-label="How hk works">
      <article>
        <span class="hk-section-number" aria-hidden="true"
          >01 / CONCURRENCY</span
        >
        <h2>Keep your tools busy.</h2>
        <p>
          Read/write locks coordinate overlapping steps. Diff and file-list
          checks let hk narrow the work that needs an exclusive lock.
        </p>
        <a href="/why-hk">How execution works →</a>
      </article>
      <article>
        <span class="hk-section-number" aria-hidden="true"
          >02 / STAGED CHANGES</span
        >
        <h2>Keep partial commits useful.</h2>
        <p>
          Stash unstaged work before fixing the staged version of a file, then
          restore it after the hook finishes.
        </p>
        <a href="/hooks#stashing-and-partial-commits">Understand stashing →</a>
      </article>
      <article>
        <span class="hk-section-number" aria-hidden="true"
          >03 / YOUR TOOLCHAIN</span
        >
        <h2>Bring the linters you use.</h2>
        <p>
          Start with built-in configurations or write a shell command. Use mise
          or your existing package manager to provide the tools.
        </p>
        <a href="/builtins">Browse builtins →</a>
      </article>
    </section>

    <section class="hk-workflow" aria-labelledby="workflow-title">
      <div>
        <p class="hk-eyebrow">From your editor to CI</p>
        <h2 id="workflow-title">A familiar command.<br />At every step.</h2>
        <p>
          Define your steps once in Pkl. Reuse them when you check a change, fix
          your working tree, or validate the whole repository.
        </p>
        <a href="/getting_started#checking-and-fixing-code"
          >Learn the workflow →</a
        >
      </div>
      <dl class="hk-commands">
        <div>
          <dt><code>hk check</code></dt>
          <dd>Check modified files</dd>
        </div>
        <div>
          <dt><code>hk fix</code></dt>
          <dd>Apply available fixes</dd>
        </div>
        <div>
          <dt><code>hk check --all</code></dt>
          <dd>Check the repository in CI</dd>
        </div>
        <div>
          <dt><code>hk check --plan</code></dt>
          <dd>Preview which steps will run</dd>
        </div>
      </dl>
    </section>

    <section class="hk-doc-links" aria-labelledby="docs-title">
      <h2 id="docs-title">Make it fit your project.</h2>
      <div>
        <a href="/configuration"
          ><strong>Configuration <span aria-hidden="true">↗</span></strong
          ><span>Files, steps, profiles, and local overrides.</span></a
        >
        <a href="/mise_integration"
          ><strong>mise integration <span aria-hidden="true">↗</span></strong
          ><span>Consistent tools in your shell and Git hooks.</span></a
        >
        <a href="/logging"
          ><strong>Troubleshooting <span aria-hidden="true">↗</span></strong
          ><span>Explain skipped steps and inspect slow runs.</span></a
        >
        <a href="/cli/"
          ><strong>CLI reference <span aria-hidden="true">↗</span></strong
          ><span>Every command, argument, and flag.</span></a
        >
      </div>
    </section>
  </div>
</template>

<style scoped>
.hk-home {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 32px 80px;
}
.hk-hero {
  display: grid;
  grid-template-columns: 1.08fr 1fr;
  align-items: center;
  gap: 64px;
  padding: 80px 0 72px;
}
.hk-eyebrow,
.hk-section-number {
  font-family: var(--vp-font-family-mono);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--vp-c-text-2);
}
.hk-hero-copy > .hk-eyebrow {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 28px;
}
.hk-hero h1 {
  font-size: clamp(40px, 4.6vw, 62px);
  font-weight: 750;
  line-height: 1.06;
  letter-spacing: -0.045em;
}
.hk-hero h1 span {
  color: var(--vp-c-brand-1);
}
.hk-intro {
  margin-top: 24px;
  max-width: 450px;
  color: var(--vp-c-text-2);
  font-size: 18px;
  line-height: 1.7;
}
.hk-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin: 28px 0;
}
.hk-button {
  display: inline-flex;
  align-items: center;
  gap: 24px;
  min-height: 46px;
  padding: 10px 18px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
}
.hk-button:hover {
  border-color: var(--vp-c-brand-1);
}
.hk-button-primary {
  background: var(--vp-c-brand-3);
  color: var(--vp-button-brand-text);
  border-color: transparent;
}
.hk-button-primary:hover {
  background: var(--vp-c-brand-2);
  color: var(--vp-button-brand-text);
}
.hk-install {
  display: flex;
  align-items: center;
  gap: 12px;
  max-width: 370px;
  padding: 10px 12px 10px 16px;
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
}
.hk-prompt {
  color: var(--vp-c-brand-1);
}
.hk-install code {
  font-size: 14px;
}
.hk-install button {
  margin-left: auto;
  padding: 4px 8px;
  font-size: 12px;
  border-radius: 4px;
  color: var(--vp-c-text-2);
}
.hk-install button:hover {
  color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-soft);
}
.hk-install-note {
  max-width: 390px;
  min-height: 40px;
  margin-top: 10px;
  font-size: 12px;
  line-height: 1.7;
  color: var(--vp-c-text-2);
}
.hk-install-note a {
  white-space: nowrap;
  color: var(--vp-c-brand-1);
}
.hk-preview {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
  border-radius: 10px;
  background: var(--vp-c-bg-soft);
  box-shadow: 0 20px 60px rgb(0 0 0 / 8%);
}
.hk-preview-header {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 22px;
  border-bottom: 1px solid var(--vp-c-divider);
  font-family: var(--vp-font-family-mono);
  font-size: 12px;
}
.hk-preview-label {
  color: var(--vp-c-text-2);
}
.hk-config {
  overflow-x: auto;
  padding: 24px;
  font-size: 13px;
  line-height: 1.9;
  tab-size: 2;
}
.hk-code-keyword {
  color: var(--hk-code-keyword);
}
.hk-code-string {
  color: var(--vp-c-brand-1);
}
.hk-coordination {
  padding: 20px 24px;
  border-top: 1px solid var(--vp-c-divider);
}
.hk-coordination > .hk-eyebrow {
  margin-bottom: 16px;
}
.hk-lane {
  display: flex;
  align-items: center;
  gap: 18px;
  margin-top: 8px;
  font-size: 11px;
}
.hk-lane > code {
  width: 58px;
  flex-shrink: 0;
  color: var(--vp-c-text-2);
}
.hk-lane > div {
  display: flex;
  flex: 1;
  gap: 4px;
  background: var(--vp-c-bg);
  border-radius: 3px;
}
.hk-lane-check,
.hk-lane-fix {
  padding: 2px 12px;
  border-radius: 3px;
  text-align: center;
}
.hk-lane-check {
  width: 45%;
  background: var(--vp-c-brand-soft);
  color: var(--vp-c-brand-1);
}
.hk-lane-long {
  width: 62%;
}
.hk-lane-fix {
  background: var(--hk-write-bg);
  color: var(--hk-code-keyword);
}
.hk-coordination > p:last-child {
  margin-top: 14px;
  color: var(--vp-c-text-2);
  font-size: 12px;
  line-height: 1.6;
}
.hk-preview-link {
  display: flex;
  justify-content: space-between;
  padding: 13px 24px;
  font-size: 12px;
  color: var(--vp-c-brand-1);
  border-top: 1px solid var(--vp-c-divider);
}
.hk-principles {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 36px;
  padding: 40px 0 56px;
  border-top: 1px solid var(--vp-c-divider);
}
.hk-principles h2 {
  margin: 16px 0 12px;
  font-size: 21px;
  letter-spacing: -0.025em;
  font-weight: 650;
}
.hk-principles p,
.hk-workflow p:not(.hk-eyebrow) {
  color: var(--vp-c-text-2);
  line-height: 1.8;
  font-size: 15px;
}
.hk-principles a,
.hk-workflow a {
  display: inline-block;
  margin-top: 16px;
  color: var(--vp-c-brand-1);
  font-size: 14px;
  font-weight: 500;
}
.hk-workflow {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 80px;
  padding: 56px 0;
  border-top: 1px solid var(--vp-c-divider);
}
.hk-workflow h2 {
  margin: 16px 0;
  font-size: 34px;
  line-height: 1.2;
  letter-spacing: -0.035em;
  font-weight: 650;
}
.hk-commands {
  align-self: center;
}
.hk-commands > div {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 6px 12px;
  padding: 20px 0;
  border-bottom: 1px solid var(--vp-c-divider);
}
.hk-commands dt {
  font-size: 14px;
  color: var(--vp-c-brand-1);
}
.hk-commands dd {
  font-size: 13px;
  color: var(--vp-c-text-2);
}
.hk-doc-links {
  padding-top: 40px;
  border-top: 1px solid var(--vp-c-divider);
}
.hk-doc-links h2 {
  font-size: 24px;
  font-weight: 650;
  letter-spacing: -0.025em;
  margin-bottom: 24px;
}
.hk-doc-links > div {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}
.hk-doc-links a {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 20px;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
}
.hk-doc-links a:hover {
  background: var(--vp-c-bg-soft);
  border-color: var(--vp-c-brand-1);
}
.hk-doc-links strong {
  display: flex;
  justify-content: space-between;
  font-size: 15px;
}
.hk-doc-links strong span {
  color: var(--vp-c-brand-1);
}
.hk-doc-links a > span {
  color: var(--vp-c-text-2);
  font-size: 14px;
}
@media (max-width: 959px) {
  .hk-hero {
    gap: 32px;
    padding-top: 56px;
  }
  .hk-config {
    font-size: 11px;
    padding: 18px;
  }
  .hk-preview-header {
    padding: 14px 18px;
  }
  .hk-principles {
    gap: 24px;
  }
  .hk-workflow {
    gap: 32px;
  }
}
@media (max-width: 719px) {
  .hk-home {
    padding: 0 24px 48px;
  }
  .hk-hero {
    grid-template-columns: 1fr;
    padding: 40px 0;
    gap: 24px;
  }
  .hk-hero h1 {
    font-size: clamp(40px, 8vw, 58px);
  }
  .hk-hero-copy > .hk-eyebrow {
    font-size: 10px;
    gap: 8px;
  }
  .hk-config {
    font-size: 12px;
  }
  .hk-principles,
  .hk-workflow,
  .hk-doc-links > div {
    grid-template-columns: 1fr;
  }
  .hk-principles {
    padding: 32px 0;
    gap: 32px;
  }
  .hk-workflow {
    padding: 32px 0;
    gap: 16px;
  }
  .hk-doc-links {
    padding-top: 32px;
  }
}
</style>
