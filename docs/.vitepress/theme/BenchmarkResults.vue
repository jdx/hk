<script setup lang="ts">
import { computed } from "vue";
import { data } from "../benchmarks.data";
import type { Scenario, Stats } from "../benchmarks.data";

const results = data;

const fmt = (s: number) =>
  s >= 10 ? `${s.toFixed(1)} s` : s >= 1 ? `${s.toFixed(2)} s` : `${Math.round(s * 1000)} ms`;

const correct = (st: Stats) => st.correct.passed === st.correct.trials;

// Two tools only differ when the gap between their medians is larger than
// either one's own spread across samples. Otherwise the page calls it level.
const separated = (a: Stats, b: Stats) =>
  Math.abs(a.median - b.median) > Math.max(a.max - a.min, b.max - b.min);

interface Row {
  key: string;
  label: string;
  mode: string;
  stats: Stats;
  ok: boolean;
}

function rows(s: Scenario): Row[] {
  return Object.entries(s.results)
    .map(([key, stats]) => ({
      key,
      label: results!.subjects[key].label,
      mode: results!.subjects[key].mode,
      stats,
      ok: correct(stats),
    }))
    .sort((a, b) => a.stats.median - b.stats.median);
}

function scale(s: Scenario) {
  return Math.max(...Object.values(s.results).map((r) => r.max));
}

// The headline compares hk with the fastest *other* tool whose output was
// right every time. A configuration that races is never the benchmark to beat.
function verdict(s: Scenario): string {
  const hk = s.results.hk;
  if (!hk) return "";
  const rivals = rows(s).filter((r) => r.key !== "hk" && r.ok);
  if (!rivals.length) return "";
  const best = rivals[0];
  const name = `${best.label} (${best.mode})`;
  if (!separated(hk, best.stats)) return `hk and ${name} are level.`;
  const ratio = best.stats.median / hk.median;
  return ratio > 1
    ? `hk is ${ratio.toFixed(1)}× faster than ${name}, the fastest other tool that produced the right files every time.`
    : `${name} is ${(1 / ratio).toFixed(1)}× faster than hk.`;
}

const scenarios = computed(() => results?.scenarios ?? []);
const measured = computed(() =>
  results ? new Date(results.generated).toISOString().slice(0, 10) : "",
);
</script>

<template>
  <div v-if="!results" class="bench-empty">
    <p>
      No verified benchmark run has been published yet. The page only shows numbers
      from a run in which every tool that is safe by design produced exactly the
      right files. Run <code>mise run benchmark</code> to measure on your own machine.
    </p>
  </div>

  <div v-else class="bench">
    <p class="bench-meta">
      Measured {{ measured }} on {{ results.machine.cpus }} CPUs
      <template v-if="results.machine.cpu">({{ results.machine.cpu }})</template>,
      {{ results.machine.os }}. hk {{ results.versions.hk }}, lefthook
      {{ results.versions.lefthook }}, pre-commit {{ results.versions["pre-commit"] }},
      prek {{ results.versions.prek }}.
      <a v-if="results.workflow_run" :href="results.workflow_run">Workflow run</a>
    </p>

    <section v-for="s in scenarios" :key="s.key" class="bench-card">
      <h3 :id="`bench-${s.key}`">{{ s.title }}</h3>
      <p class="bench-summary">{{ s.summary }}</p>
      <p v-if="verdict(s)" class="bench-verdict">{{ verdict(s) }}</p>

      <ul class="bench-bars" role="list">
        <li v-for="r in rows(s)" :key="r.key" :class="{ hk: r.key === 'hk', wrong: !r.ok }">
          <span class="bench-name">
            <strong>{{ r.label }}</strong>
            <small>{{ r.mode }}</small>
          </span>
          <span class="bench-track" aria-hidden="true">
            <span
              class="bench-whisker"
              :style="{
                left: `${(r.stats.min / scale(s)) * 100}%`,
                width: `${((r.stats.max - r.stats.min) / scale(s)) * 100}%`,
              }"
            />
            <span class="bench-bar" :style="{ width: `${(r.stats.median / scale(s)) * 100}%` }" />
          </span>
          <span class="bench-value">{{ fmt(r.stats.median) }}</span>
          <span v-if="r.ok" class="bench-ok">✓ correct</span>
          <span v-else class="bench-bad">
            ✗ wrong files in {{ r.stats.correct.trials - r.stats.correct.passed }}/{{ r.stats.correct.trials }} runs
          </span>
        </li>
      </ul>

      <details>
        <summary>Numbers</summary>
        <table>
          <thead>
            <tr>
              <th>Tool</th><th>Mode</th><th>Median</th><th>Mean ± σ</th><th>Range</th><th>Runs</th><th>Correct</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in rows(s)" :key="r.key">
              <td>{{ r.label }}</td>
              <td>{{ r.mode }}</td>
              <td>{{ fmt(r.stats.median) }}</td>
              <td>{{ fmt(r.stats.mean) }} ± {{ fmt(r.stats.stddev) }}</td>
              <td>{{ fmt(r.stats.min) }}–{{ fmt(r.stats.max) }}</td>
              <td>{{ r.stats.runs }}</td>
              <td>
                {{ r.stats.correct.passed }}/{{ r.stats.correct.trials }}
                <template v-if="!r.ok">(up to {{ r.stats.correct.max_wrong_files }} files wrong)</template>
              </td>
            </tr>
          </tbody>
        </table>
      </details>
    </section>
  </div>
</template>

<style scoped>
.bench-empty,
.bench-card {
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  background: var(--vp-c-bg-soft);
  padding: 4px 20px 12px;
  margin: 20px 0;
}
.bench-meta,
.bench-summary {
  color: var(--vp-c-text-2);
  font-size: 14px;
}
.bench-card h3 {
  margin-top: 16px;
}
.bench-verdict {
  font-weight: 600;
}
.bench-bars {
  list-style: none;
  padding: 0;
  margin: 12px 0;
}
.bench-bars li {
  display: grid;
  grid-template-columns: minmax(150px, 1.2fr) 3fr 70px minmax(150px, 1fr);
  align-items: center;
  gap: 12px;
  margin: 8px 0;
  font-size: 14px;
}
.bench-name {
  display: flex;
  flex-direction: column;
  line-height: 1.3;
}
.bench-name small {
  color: var(--vp-c-text-3);
}
.bench-track {
  position: relative;
  height: 18px;
}
.bench-bar {
  position: absolute;
  inset: 3px auto 3px 0;
  border-radius: 3px;
  background: var(--vp-c-text-3);
  opacity: 0.55;
}
.hk .bench-bar {
  background: var(--vp-c-brand-1);
  opacity: 1;
}
.wrong .bench-bar {
  background: repeating-linear-gradient(
    135deg,
    var(--vp-c-danger-1) 0 4px,
    transparent 4px 8px
  );
  opacity: 0.8;
}
.bench-whisker {
  position: absolute;
  top: 8px;
  height: 2px;
  background: var(--vp-c-text-2);
}
.bench-value {
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.bench-ok {
  color: var(--vp-c-success-1);
}
.bench-bad {
  color: var(--vp-c-danger-1);
}
@media (max-width: 640px) {
  .bench-bars li {
    grid-template-columns: 1fr 70px;
  }
  .bench-track,
  .bench-ok,
  .bench-bad {
    grid-column: 1 / -1;
  }
}
</style>
