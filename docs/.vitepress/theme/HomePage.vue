<script setup lang="ts">
import { useData } from "vitepress";
import { computed, onUnmounted, ref } from "vue";
import HomeShowreel from "./HomeShowreel.vue";
import ShantyChip from "./ShantyChip.vue";
import { data as pirateData } from "../pirate.data";
import { PIRATE_LOCALE, pirateLink } from "./shanty-mode";

// The landing page serves both locales: docs/index.md, and docs/pirate/index.md
// for sea shanty mode (shanty-mode.ts), which the server renders as it is, so
// its words and links are the pirate ones from the first byte. The pirate page
// is the same page with the crew's words, plus a few parts of its own (the
// chart in the hero, the voyage, the closing chorus); shanty-home.css draws it.
const { localeIndex } = useData();
const pirate = computed(() => localeIndex.value === PIRATE_LOCALE);
/** A link to a page, in this landing page's locale. */
const to = (path: string) => (pirate.value ? pirateLink(path, pirateData.missing) : path);

const installCommand = "mise use hk";
const copyStatus = ref("");
let copyTimer: ReturnType<typeof setTimeout> | undefined;

async function copyInstall() {
  clearTimeout(copyTimer);
  try {
    await navigator.clipboard.writeText(installCommand);
    copyStatus.value = pirate.value ? "Copied, matey! 'Tis on yer clipboard." : "Copied to clipboard";
  } catch {
    copyStatus.value = pirate.value ? "Select the command and copy it by hand, matey." : "Select the command to copy it manually";
  }
  copyTimer = setTimeout(() => {
    copyStatus.value = "";
  }, 4000);
}

onUnmounted(() => clearTimeout(copyTimer));

// The hero's sea chart: sixteen rhumb lines through the compass rose, one every
// 11.25 degrees, run far past the chart's edges (the chart clips them). In the
// rose's own units, where its outer ring has a radius of 96.
const RHUMBS = Array.from({ length: 16 }, (_, i) => {
  const a = (i * Math.PI) / 16;
  const x = Math.round(Math.cos(a) * 1600);
  const y = Math.round(Math.sin(a) * 1600);
  return `M${-x} ${-y}L${x} ${y}`;
}).join("");
</script>

<template>
  <div class="hk-home">
    <section class="hk-hero" :class="{ 'hk-sea-chart': pirate }" aria-labelledby="hero-title">
      <!-- The chart the hero is drawn on: rhumb lines and a compass rose. -->
      <div v-if="pirate" class="hk-sea-chart-art" aria-hidden="true">
        <svg class="hk-sea-rose" viewBox="-100 -100 200 200">
          <path class="hk-sea-rhumbs" :d="RHUMBS" />
          <circle class="hk-sea-rose-face" r="96" />
          <circle r="96" />
          <circle r="89" />
          <circle class="hk-sea-rose-ticks" r="92.5" />
          <circle r="60" stroke-dasharray="1.5 3.5" />
          <g v-for="a in [22.5, 67.5, 112.5, 157.5, 202.5, 247.5, 292.5, 337.5]" :key="a" :transform="`rotate(${a})`">
            <path class="hk-sea-rose-dark" d="M0-46L-5-5 0 0Z" />
            <path class="hk-sea-rose-light" d="M0-46L5-5 0 0Z" />
          </g>
          <g v-for="a in [45, 135, 225, 315]" :key="a" :transform="`rotate(${a})`">
            <path class="hk-sea-rose-dark" d="M0-66L-8-8 0 0Z" />
            <path class="hk-sea-rose-light" d="M0-66L8-8 0 0Z" />
          </g>
          <g v-for="a in [0, 90, 180, 270]" :key="a" :transform="`rotate(${a})`">
            <path class="hk-sea-rose-dark" d="M0-88L-11-11 0 0Z" />
            <path class="hk-sea-rose-light" d="M0-88L11-11 0 0Z" />
          </g>
          <circle class="hk-sea-rose-light" r="6" />
          <!-- North, marked as old charts mark it, above the ring. -->
          <path class="hk-sea-rose-dark" d="M0-112l7 11H-7Z" />
        </svg>
      </div>
      <div class="hk-hero-copy">
        <ShantyChip :pirate="pirate" />
        <p v-if="pirate" class="hk-sea-cartouche">Git hooks fer linters and formatters</p>
        <h1 id="hero-title">
          <template v-if="pirate">All hands haul <em>at once</em></template>
          <template v-else>Git hooks for linters and formatters</template>
        </h1>
        <p v-if="pirate" class="hk-intro">
          Chart yer checks once, and hk pipes the crew on deck afore every
          commit, while ye work, or in CI. Independent steps haul in parallel,
          and a lock on each file takes the strain wherever two hands share one.
        </p>
        <p v-else class="hk-intro">
          Configure your checks once and run them before commits, while you work,
          or in CI. hk runs independent steps in parallel and coordinates changes
          to shared files.
        </p>
        <div class="hk-actions">
          <a class="hk-button hk-button-primary" :href="to('/getting_started')"
            ><template v-if="pirate">Set sail</template
            ><template v-else>Get started</template>
            <span aria-hidden="true">→</span></a
          >
          <a class="hk-button" :href="to('/reference/examples/')"
            ><template v-if="pirate">See how other ships be rigged</template
            ><template v-else>Explore configurations</template></a
          >
        </div>
        <div class="hk-install">
          <span class="hk-prompt" aria-hidden="true">$</span>
          <code>{{ installCommand }}</code>
          <button
            type="button"
            :aria-label="pirate ? 'Nab it: copy the mise install command' : 'Copy mise install command'"
            @click="copyInstall"
          >
            {{ pirate ? "Nab it" : "Copy" }}
          </button>
        </div>
        <p class="hk-install-note" role="status">
          {{
            copyStatus ||
            (pirate
              ? "mise, the quartermaster, brings hk aboard, or pick another way to sign on."
              : "Install with mise, or choose another installation method.")
          }}
          <a v-if="!copyStatus" :href="to('/getting_started#installation')"
            >{{ pirate ? "Every way aboard" : "All options" }} →</a
          >
        </p>
      </div>

      <div class="hk-preview">
        <div class="hk-preview-header">
          <span>hk.pkl</span
          ><span class="hk-preview-label">{{ pirate ? "The articles we sail by" : "One set of steps" }}</span>
        </div>
        <pre
          class="hk-config"
          role="group"
          :aria-label="pirate ? 'The ship\'s articles: steps shared by the check, fix, and pre-commit hooks' : 'Example steps shared by the check, fix, and pre-commit hooks'"
        ><code><span class="hk-code-comment">// Run by check, fix, and pre-commit</span>
steps {
  [<span class="hk-code-string">"prettier"</span>] = Builtins.prettier
  [<span class="hk-code-string">"eslint"</span>] = Builtins.eslint
  [<span class="hk-code-string">"ruff"</span>] = Builtins.ruff
}</code></pre>
        <div class="hk-coordination">
          <p class="hk-eyebrow">{{ pirate ? "Side by side, each on a file o' their own" : "Independent files, concurrent work" }}</p>
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
          <p v-if="pirate">
            Every file hauls in its own lane, all at once. Checks can share a
            file; a fix makes its file fast until the mending's done.
          </p>
          <p v-else>
            Checks can read a file concurrently; fixes wait for exclusive access.
          </p>
        </div>
        <a
          class="hk-preview-link"
          :href="to('/getting_started#your-first-configuration')"
          >{{ pirate ? "Unroll the whole configuration" : "See the complete configuration" }} <span aria-hidden="true">↗</span></a
        >
      </div>
    </section>

    <!-- What one commit goes through, as the shanty's verses tell it. -->
    <section v-if="pirate" class="hk-sea-voyage" aria-labelledby="voyage-title">
      <div>
        <h2 id="voyage-title">The voyage o' every commit</h2>
        <p>What the crew does on every run o' the pre-commit hook, verse by verse.</p>
      </div>
      <ol>
        <li>
          <span class="hk-sea-mark" aria-hidden="true">
            <svg class="hk-sea-icon" viewBox="0 0 24 24">
              <circle cx="12" cy="4.5" r="2" />
              <path d="M12 6.5V21M8.5 10h7M5 14.5c.6 3.6 3.4 6.5 7 6.5s6.4-2.9 7-6.5M3.6 16 5 14.5 6.6 16M17.4 16 19 14.5 20.4 16" />
            </svg>
          </span>
          <strong><code>git commit</code></strong>
          <p>Ye christen her, hit return, and she weighs anchor.</p>
        </li>
        <li>
          <span class="hk-sea-mark" aria-hidden="true">
            <svg class="hk-sea-icon" viewBox="0 0 24 24">
              <path d="M12 2.5v2M9 4.5h6M9 4.5c0 5-1 9-3.5 12.5h13C16 13.5 15 9.5 15 4.5M4.5 17h15" />
              <circle cx="12" cy="19.5" r="1.5" />
            </svg>
          </span>
          <strong>All hands on deck</strong>
          <p>The pre-commit hook pipes the crew up from stem to stern.</p>
        </li>
        <li>
          <span class="hk-sea-mark" aria-hidden="true">
            <svg class="hk-sea-icon" viewBox="0 0 24 24">
              <path d="M4 11h16v8.5H4ZM4 11c0-3 3.6-5 8-5s8 2 8 5M4 13.5h16M10.5 12.5h3v3h-3Z" />
            </svg>
          </span>
          <strong>Snug in the hold</strong>
          <p>hk can stow yer unstaged work below, so the crew sees only the cargo ye ship.</p>
        </li>
        <li>
          <span class="hk-sea-mark" aria-hidden="true">
            <svg class="hk-sea-icon" viewBox="0 0 24 24">
              <path d="M2.5 12.5h19l-3 4.5h-13ZM6 4.5l4 12.5M11 4.5l4 12.5M16 4.5l4 12.5M2 21q2.5-1.6 5 0t5 0 5 0 5 0" />
            </svg>
          </span>
          <strong>All hands haul at once</strong>
          <p>Lookouts and sailmakers work in parallel, and a lock on each file takes the strain.</p>
        </li>
        <li>
          <span class="hk-sea-mark" aria-hidden="true">
            <svg class="hk-sea-icon" viewBox="0 0 24 24">
              <path d="M12 2.5v14M13.5 4l6 10.5h-6M10.5 6.5 5.5 14.5h5M3 17h18l-2.5 4h-13Z" />
            </svg>
          </span>
          <strong>Bound for the main</strong>
          <p>
            The fixes are staged, yer stowed work comes back up, and the commit
            is made. A squall no fixer can mend hauls her back to port.
          </p>
        </li>
      </ol>
    </section>

    <!-- Full width under the hero; left out of builds without a render to show. -->
    <HomeShowreel :shanty="pirate" />

    <section class="hk-principles" :aria-label="pirate ? 'How the crew works' : 'How hk works'">
      <article>
        <svg v-if="pirate" class="hk-sea-art" viewBox="0 0 200 110" aria-hidden="true">
          <path class="hk-sea-art-sea" d="M0 92q12.5-6 25 0t25 0 25 0 25 0 25 0 25 0 25 0 25 0M12 104q12.5-5 25 0t25 0 25 0 25 0 25 0 25 0 25 0" />
          <g v-for="x in [48, 74, 100, 126, 152]" :key="x">
            <path :d="`M${x + 3} 46L${x - 20} 98M${x} 41v21`" />
            <path class="hk-sea-art-blade" :d="`M${x - 16.5} 90l-3.5 8`" />
            <circle class="hk-sea-art-fill" :cx="x" cy="35" r="5.5" />
          </g>
          <path class="hk-sea-art-fill" d="M22 62h156c-4 10-14 18-28 20H50c-14-2-24-10-28-20Z" />
          <path d="M30 70h140M178 62V28l15 6-15 6" />
        </svg>
        <h2>{{ pirate ? "Side by side, made fast" : "Parallel execution" }}</h2>
        <p v-if="pirate">
          Independent hands haul in parallel, even over shared cargo:
          read/write locks make each file fast, so no two collide. Where a tool
          can call out a patch or a list o' files, hk narrows the work that
          needs a file to itself.
        </p>
        <p v-else>
          Read/write locks coordinate overlapping steps. Diff and file-list
          checks let hk narrow the work that needs an exclusive lock.
        </p>
        <a :href="to('/why-hk')">{{ pirate ? "How the crew hauls" : "How execution works" }} →</a>
      </article>
      <article>
        <svg v-if="pirate" class="hk-sea-art" viewBox="0 0 200 110" aria-hidden="true">
          <path class="hk-sea-art-sea" d="M0 58q10-5 20 0M180 58q10-5 20 0" />
          <path class="hk-sea-art-fill" d="M20 22h160v10c0 34-32 58-80 66-48-8-80-32-80-66Z" />
          <path d="M20 32h160M100 22v-14l14 5-14 5" />
          <path class="hk-sea-art-sea" d="M26 58h148" stroke-dasharray="2 5" />
          <path class="hk-sea-art-soft" d="M78 68h44v18H78ZM78 68c0-7 10-11 22-11s22 4 22 11M88 58v28M112 58v28" />
          <path d="M97 68h6v6h-6Z" />
          <path class="hk-sea-art-soft" d="M46 66h16c2 7 2 14 0 20H46c-2-6-2-13 0-20ZM45 72h18M45 80h18" />
          <path d="M146 32v8M141 40h10l-1.5 12h-7ZM143 52h6" />
          <circle class="hk-sea-art-glow" cx="146" cy="46" r="9" />
        </svg>
        <h2>{{ pirate ? "Snug in the hold" : "Partial commits" }}</h2>
        <p v-if="pirate">
          hk stows yer unstaged work in the hold afore it mends the staged
          version o' a file, then brings it back up when the hook is done.
        </p>
        <p v-else>
          Stash unstaged work before fixing the staged version of a file, then
          restore it after the hook finishes.
        </p>
        <a :href="to('/hooks#stashing-and-partial-commits')">{{ pirate ? "How the stowin' works" : "Understand stashing" }} →</a>
      </article>
      <article>
        <svg v-if="pirate" class="hk-sea-art" viewBox="0 0 200 110" aria-hidden="true">
          <path class="hk-sea-art-sea" d="M118 88h82" />
          <path d="M62 92V6M50 36 26 92M74 36l24 56M34 62h56" />
          <path class="hk-sea-art-fill" d="M36 62q26 9 52 0M8 92h112l-9 14H17Z" />
          <path class="hk-sea-art-sea" d="M0 104q10-5 20 0t20 0 20 0 20 0 20 0 20 0 20 0 20 0 20 0 20 0" />
          <path class="hk-sea-art-fill" d="M48 28h28l-3 12H51Z" />
          <circle class="hk-sea-art-fill" cx="64" cy="21" r="5.5" />
          <path class="hk-sea-art-fill" d="M68 19l30-8 1.6 5.5-30 8Z" />
          <path class="hk-sea-art-sea" d="M101 15l55 58" stroke-dasharray="2 5" />
          <path class="hk-sea-art-fill" d="M154 84h24l-4 4h-16Z" />
          <path d="M166 84V68M167 69l8 13h-8M165 71l-6 11h6" />
          <path class="hk-sea-art-sea" d="M122 42q4-4 8 0 4-4 8 0M138 30q3-3 6 0 3-3 6 0" />
        </svg>
        <h2>{{ pirate ? "The standing crew" : "Linter configuration" }}</h2>
        <p v-if="pirate">
          Sign on a builtin from the standing crew, or give yer own orders as a
          shell command. mise, the quartermaster, or yer usual package manager
          provisions the tools.
        </p>
        <p v-else>
          Start with built-in configurations or write a shell command. Use mise
          or your existing package manager to provide the tools.
        </p>
        <a :href="to('/builtins')">{{ pirate ? "Muster the builtins" : "Browse builtins" }} →</a>
      </article>
    </section>

    <section class="hk-workflow" aria-labelledby="workflow-title">
      <div>
        <p v-if="pirate" class="hk-sea-kicker">They steer by the one set of charts</p>
        <h2 id="workflow-title">{{ pirate ? "One set o' charts, on deck and in CI" : "Run checks locally and in CI" }}</h2>
        <p v-if="pirate">
          Chart yer steps once in Pkl. The hook, the helm and the harbour-master
          all muster the self-same crew: to check a change, mend yer working
          tree, or inspect the whole ship.
        </p>
        <p v-else>
          Define your steps once in Pkl. Reuse them when you check a change, fix
          your working tree, or validate the whole repository.
        </p>
        <a :href="to('/getting_started#checking-and-fixing-code')"
          >{{ pirate ? "Learn the ropes" : "Learn the workflow" }} →</a
        >
      </div>
      <dl class="hk-commands">
        <div>
          <dt><code>hk check</code></dt>
          <dd v-if="pirate" class="hk-sea-call">Who'll check the cargo?</dd>
          <dd>{{ pirate ? "Inspects the cargo ye've changed" : "Check modified files" }}</dd>
        </div>
        <div>
          <dt><code>hk fix</code></dt>
          <dd v-if="pirate" class="hk-sea-call">Who'll mend the canvas?</dd>
          <dd>{{ pirate ? "Makes every mend the crew knows how" : "Apply available fixes" }}</dd>
        </div>
        <div>
          <dt><code>hk check --all</code></dt>
          <dd v-if="pirate" class="hk-sea-call">Who'll haul the halyard?</dd>
          <dd>{{ pirate ? "The whole ship, fer the harbour-master in CI" : "Check the repository in CI" }}</dd>
        </div>
        <div>
          <dt><code>hk check --plan</code></dt>
          <dd v-if="pirate" class="hk-sea-call">And where are we bound?</dd>
          <dd>{{ pirate ? "The passage plan: which hands will be called" : "Preview which steps will run" }}</dd>
        </div>
      </dl>
    </section>

    <section class="hk-doc-links" aria-labelledby="docs-title">
      <p v-if="pirate" class="hk-sea-kicker">Charts fer every passage</p>
      <h2 id="docs-title">{{ pirate ? "The chart locker" : "Configuration and reference" }}</h2>
      <div>
        <a :href="to('/configuration')"
          ><strong>{{ pirate ? "Riggin' the ship" : "Configuration" }} <span aria-hidden="true">↗</span></strong
          ><span>{{ pirate ? "Configuration: files, steps, profiles (the watches) and local overrides." : "Files, steps, profiles, and local overrides." }}</span></a
        >
        <a :href="to('/mise_integration')"
          ><strong>{{ pirate ? "The quartermaster" : "mise integration" }} <span aria-hidden="true">↗</span></strong
          ><span>{{ pirate ? "mise integration: the same tools in yer shell and yer Git hooks." : "Consistent tools in your shell and Git hooks." }}</span></a
        >
        <a :href="to('/logging')"
          ><strong>{{ pirate ? "Readin' the ship's log" : "Troubleshooting" }} <span aria-hidden="true">↗</span></strong
          ><span>{{ pirate ? "Troubleshooting: why a hand sat out, and what slowed the voyage." : "Explain skipped steps and inspect slow runs." }}</span></a
        >
        <a :href="to('/cli/')"
          ><strong>{{ pirate ? "The bosun's calls" : "CLI reference" }} <span aria-hidden="true">↗</span></strong
          ><span>{{ pirate ? "CLI reference: every command, argument, and flag." : "Every command, argument, and flag." }}</span></a
        >
      </div>
    </section>

    <!-- The shanty's outro, which ends on the install command. -->
    <section v-if="pirate" class="hk-sea-outro" aria-labelledby="outro-title">
      <p class="hk-sea-kicker">So all you hands who would sign aboard,</p>
      <h2 id="outro-title">There's but three words left to say</h2>
      <p class="hk-sea-outro-words"><code>{{ installCommand }}</code></p>
      <p class="hk-sea-outro-last">And we're bound away for the main!</p>
      <a class="hk-button hk-button-primary" :href="to('/getting_started')">Set sail <span aria-hidden="true">→</span></a>
      <svg class="hk-sea-ship" viewBox="0 0 160 104" aria-hidden="true">
        <path d="M48 63V16M80 62V4M112 60V14M144 59l15-9M80 4l13 4.5L80 13" />
        <path class="hk-sea-art-fill" d="M113 16q25 18 45 35l-32 6Z" />
        <path
          class="hk-sea-art-fill"
          d="M38 22h20q6 7 2 14H36q4-7 2-14ZM36 40h24q6 8 2 16H34q4-8 2-16ZM66 12h28q7 10 2 20H64q4-10 2-20ZM62 36h36q8 11 2 22H60q5-11 2-22ZM101 18h22q6 8 2 16H99q4-8 2-16ZM98 38h28q7 9 2 18H96q4-9 2-18Z"
        />
        <path class="hk-sea-art-fill" d="M14 62l132-4c-5 15-18 23-36 25H52c-19-2-33-10-38-21Z" />
        <path d="M22 68l116-3" />
        <path class="hk-sea-art-sea" d="M0 90q10-6 20 0t20 0 20 0 20 0 20 0 20 0 20 0 20 0M10 98q10-5 20 0t20 0 20 0 20 0 20 0 20 0 20 0" />
      </svg>
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
.hk-code-comment {
  color: var(--vp-c-text-2);
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
