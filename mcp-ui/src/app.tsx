import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import logoUrl from "../../docs/public/logo.svg";
import type { ToolCaller } from "./bridge";
import type { Diagnostic, RunSnapshot, TextPage } from "./types";

const terminal = new Set(["succeeded", "failed", "cancelled"]);
const pageSize = 65536;
const pagesPerRefresh = 8;
const reviewTabs = ["overview", "diagnostics", "logs", "patch"] as const;
type ReviewTab = (typeof reviewTabs)[number];

function preferredReviewTab(run: RunSnapshot): ReviewTab {
  const diagnosticCount = (run.result?.steps ?? []).reduce(
    (count, step) => count + (step.diagnostics?.length ?? 0),
    0,
  );
  if (diagnosticCount > 0) return "diagnostics";
  if (run.has_diff) return "patch";
  return "overview";
}

async function readPages(
  call: ToolCaller,
  tool: "get_output" | "get_diff",
  runId: string,
) {
  let offset = 0;
  let text = "";
  let page: TextPage = {};
  for (let index = 0; index < pagesPerRefresh; index += 1) {
    page = (await call(tool, {
      run_id: runId,
      offset,
      limit: pageSize,
    })) as TextPage;
    text += page.text ?? "";
    const next = page.next_offset ?? offset + (page.text?.length ?? 0);
    if (page.eof !== false || next <= offset) break;
    offset = next;
  }
  const loaded = page.next_offset ?? offset;
  if (page.eof === false) {
    text += `\n… ${Math.max(0, (page.total_bytes ?? loaded) - loaded)} more bytes available from the server`;
  }
  if (page.truncated) text += "\n… capture truncated by server";
  return text;
}

export function Dashboard({
  initial,
  call,
}: {
  initial: RunSnapshot;
  call: ToolCaller;
}) {
  const [run, setRun] = useState(initial);
  const [logs, setLogs] = useState("");
  const [diff, setDiff] = useState("");
  const [diffLoading, setDiffLoading] = useState(initial.has_diff);
  const [diffError, setDiffError] = useState("");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [updated, setUpdated] = useState(Date.now());
  const [refreshError, setRefreshError] = useState("");
  const [actionError, setActionError] = useState("");
  const [activeTab, setActiveTab] = useState<ReviewTab>(() =>
    preferredReviewTab(initial),
  );
  const activeRunId = useRef(initial.id);
  const generation = useRef(0);
  const refreshSequence = useRef(0);
  const wasTerminal = useRef(terminal.has(initial.status));

  useEffect(() => {
    generation.current += 1;
    activeRunId.current = initial.id;
    setRun(initial);
    setLogs("");
    setDiff("");
    setDiffLoading(initial.has_diff);
    setDiffError("");
    setQuery("");
    setRefreshError("");
    setActionError("");
    setActiveTab(preferredReviewTab(initial));
    wasTerminal.current = terminal.has(initial.status);
    setUpdated(Date.now());
  }, [initial.id]);

  const refresh = async (runId = activeRunId.current) => {
    const requestGeneration = generation.current;
    const requestSequence = ++refreshSequence.current;
    try {
      const next = (await call("get_run", { run_id: runId })) as RunSnapshot;
      const output = await readPages(call, "get_output", runId);
      if (
        requestGeneration !== generation.current ||
        requestSequence !== refreshSequence.current ||
        activeRunId.current !== runId
      )
        return;
      setRun(next);
      setLogs(output);
      setUpdated(Date.now());
      setRefreshError("");
      if (!next.has_diff) {
        setDiff("");
        setDiffLoading(false);
        setDiffError("");
        return;
      }
      setDiffLoading(true);
      setDiffError("");
      try {
        const patch = await readPages(call, "get_diff", runId);
        if (
          requestGeneration !== generation.current ||
          requestSequence !== refreshSequence.current ||
          activeRunId.current !== runId
        )
          return;
        setDiff(patch);
        setDiffLoading(false);
        setUpdated(Date.now());
      } catch (reason) {
        if (
          requestGeneration !== generation.current ||
          requestSequence !== refreshSequence.current ||
          activeRunId.current !== runId
        )
          return;
        const message =
          reason instanceof Error ? reason.message : String(reason);
        setDiffLoading(false);
        setDiffError(message);
        setRefreshError(message);
      }
    } catch (reason) {
      if (
        requestGeneration !== generation.current ||
        requestSequence !== refreshSequence.current ||
        activeRunId.current !== runId
      )
        return;
      const message = reason instanceof Error ? reason.message : String(reason);
      setRefreshError(message);
    }
  };

  useEffect(() => {
    void refresh();
    if (terminal.has(run.status)) return;
    const timer = window.setInterval(() => void refresh(), 1000);
    return () => window.clearInterval(timer);
  }, [run.id, run.status]);

  const steps = run.result?.steps ?? [];
  const passedSteps = steps.filter((step) => step.status === "passed");
  const visibleSteps = steps.filter((step) => step.status !== "passed");
  const diagnostics = steps.flatMap((step) => step.diagnostics ?? []);
  const diagnosticsByFile = useMemo(() => {
    const grouped = new Map<string, Diagnostic[]>();
    for (const diagnostic of diagnostics) {
      const file = diagnostic.path ?? "Project";
      grouped.set(file, [...(grouped.get(file) ?? []), diagnostic]);
    }
    return grouped;
  }, [diagnostics]);
  const visibleLogs = logs
    .split("\n")
    .filter((line) => line.toLowerCase().includes(query.toLowerCase()))
    .join("\n");
  const stale = !terminal.has(run.status) && Date.now() - updated > 5000;
  const isTerminal = terminal.has(run.status);
  const completedSteps = steps.filter((step) =>
    ["passed", "failed", "skipped", "cancelled"].includes(step.status),
  ).length;
  const progress = steps.length
    ? Math.round((completedSteps / steps.length) * 100)
    : 0;

  useEffect(() => {
    if (!wasTerminal.current && isTerminal) {
      setActiveTab(preferredReviewTab(run));
    }
    wasTerminal.current = isTerminal;
  }, [isTerminal, run]);

  const start = async (tool: "start_safe_check" | "start_safe_fix") => {
    if (
      tool === "start_safe_fix" &&
      !window.confirm(
        "Run effects-checked fixes and review the resulting patch?",
      )
    )
      return;
    setBusy(true);
    setActionError("");
    const requestGeneration = ++generation.current;
    try {
      const next = (await call(tool, { root: run.root })) as RunSnapshot;
      if (requestGeneration !== generation.current) return;
      activeRunId.current = next.id;
      setRun(next);
      setLogs("");
      setDiff("");
      setDiffLoading(next.has_diff);
      setDiffError("");
      setQuery("");
      setActiveTab("overview");
      setRefreshError("");
      setActionError("");
    } catch (reason) {
      if (requestGeneration === generation.current)
        setActionError(
          reason instanceof Error ? reason.message : String(reason),
        );
    } finally {
      setBusy(false);
    }
  };

  const cancel = async () => {
    setActionError("");
    try {
      await call("cancel_run", { run_id: activeRunId.current });
      await refresh();
    } catch (reason) {
      setActionError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  const alert =
    actionError ||
    refreshError ||
    run.error ||
    (stale ? "Run data may be stale—waiting for the server." : "");

  return (
    <main class={isTerminal ? "complete" : "running"}>
      <header class="topbar">
        <div class="identity">
          <img class="mark" src={logoUrl} alt="hk" />
          <div>
            <h1>{run.kind.replaceAll("_", " ")}</h1>
            <p class="root" title={run.root}>
              {run.root}
            </p>
          </div>
        </div>
        <div class="run-meta">
          {isTerminal && <time>{run.result?.duration_ms ?? 0} ms</time>}
          <span class={`status ${run.status}`} aria-live="polite">
            <i />
            {run.status}
          </span>
          <button
            class="icon-button"
            aria-label="Refresh"
            onClick={() => void refresh()}
          >
            ↻
          </button>
        </div>
      </header>

      {alert && <aside role="alert">{alert}</aside>}

      {!isTerminal ? (
        <>
          <section class="running-summary" aria-label="Run progress">
            <div>
              <p class="eyebrow">Checking project</p>
              <h2>
                {steps.length
                  ? `${completedSteps} of ${steps.length} steps complete`
                  : "Preparing execution plan…"}
              </h2>
            </div>
            <strong>{steps.length ? `${progress}%` : "Live"}</strong>
          </section>
          <div
            class={`progress ${steps.length ? "" : "indeterminate"}`}
            aria-label={
              steps.length ? `${progress}% complete` : "Run in progress"
            }
          >
            <i style={{ width: `${progress}%` }} />
          </div>

          <div class="live-grid">
            <section class="live-steps">
              <h3>Execution</h3>
              <ol>
                {visibleSteps.map((step) => (
                  <li key={step.name} class={step.status}>
                    <span class={`dot ${step.status}`} />
                    <span>
                      <strong>{step.name}</strong>
                      <small>
                        {step.effects
                          ?.map((effect) => effect.effect ?? "unknown")
                          .join(" · ") || step.status}
                      </small>
                    </span>
                    <time>{step.duration_ms} ms</time>
                  </li>
                ))}
                {!steps.length && (
                  <li class="empty">Waiting for structured step results…</li>
                )}
              </ol>
              {passedSteps.length > 0 && (
                <details class="passed-steps">
                  <summary>
                    {passedSteps.length} passed{" "}
                    {passedSteps.length === 1 ? "step" : "steps"}
                  </summary>
                  <ol>
                    {passedSteps.map((step) => (
                      <li key={step.name} class={step.status}>
                        <span class={`dot ${step.status}`} />
                        <span>
                          <strong>{step.name}</strong>
                          <small>
                            {step.effects
                              ?.map((effect) => effect.effect ?? "unknown")
                              .join(" · ") || step.status}
                          </small>
                        </span>
                        <time>{step.duration_ms} ms</time>
                      </li>
                    ))}
                  </ol>
                </details>
              )}
            </section>
            <section class="live-output">
              <div class="section-heading">
                <h3>Live output</h3>
                <span>Following</span>
              </div>
              <pre tabIndex={0}>{logs || "Waiting for output…"}</pre>
            </section>
          </div>

          <footer class="live-footer">
            <span>
              <i /> Results may change while the run is active
            </span>
            <button onClick={() => void cancel()}>Cancel run</button>
          </footer>
        </>
      ) : (
        <>
          <div class="reviewbar">
            <div class="tabs" role="tablist" aria-label="Completed run details">
              {reviewTabs.map((tab) => (
                <button
                  id={`tab-${tab}`}
                  type="button"
                  role="tab"
                  aria-controls={`panel-${tab}`}
                  aria-selected={activeTab === tab}
                  tabIndex={activeTab === tab ? 0 : -1}
                  onClick={() => setActiveTab(tab)}
                  onKeyDown={(event) => {
                    const index = reviewTabs.indexOf(tab);
                    let next: ReviewTab | undefined;
                    if (event.key === "ArrowRight") {
                      next = reviewTabs[(index + 1) % reviewTabs.length];
                    } else if (event.key === "ArrowLeft") {
                      next =
                        reviewTabs[
                          (index - 1 + reviewTabs.length) % reviewTabs.length
                        ];
                    } else if (event.key === "Home") {
                      next = reviewTabs[0];
                    } else if (event.key === "End") {
                      next = reviewTabs[reviewTabs.length - 1];
                    }
                    if (!next) return;
                    event.preventDefault();
                    setActiveTab(next);
                    requestAnimationFrame(() =>
                      document.getElementById(`tab-${next}`)?.focus(),
                    );
                  }}
                  key={tab}
                >
                  {tab}
                  {tab === "diagnostics" && diagnostics.length > 0 && (
                    <b>{diagnostics.length}</b>
                  )}
                  {tab === "patch" && diff && <b>•</b>}
                </button>
              ))}
            </div>
            <nav aria-label="Run actions">
              <button
                disabled={busy}
                aria-label="Rerun safe check"
                onClick={() => void start("start_safe_check")}
              >
                Rerun
              </button>
              <button
                class="primary"
                disabled={busy}
                aria-label="Confirm safe fix"
                onClick={() => void start("start_safe_fix")}
              >
                Fix safely
              </button>
            </nav>
          </div>

          <section
            id="panel-overview"
            class="review-panel"
            role="tabpanel"
            aria-labelledby="tab-overview"
            hidden={activeTab !== "overview"}
          >
            {run.status === "failed" && !run.result && (
              <section class="run-failure" role="alert">
                <div>
                  <strong>Run failed before step results were available</strong>
                  <p>
                    Open the captured logs for the configuration or setup error.
                  </p>
                </div>
                <button type="button" onClick={() => setActiveTab("logs")}>
                  View logs
                </button>
              </section>
            )}
            <div class="summary-strip" aria-label="Run summary">
              <div>
                <span>Steps</span>
                <strong>{steps.length}</strong>
                <small>
                  {steps.filter((step) => step.status === "passed").length}{" "}
                  passed
                </small>
              </div>
              <div>
                <span>Diagnostics</span>
                <strong>{diagnostics.length}</strong>
                <small>{diagnostics.length ? "needs review" : "clean"}</small>
              </div>
              <div>
                <span>Effects</span>
                <strong>
                  {[
                    ...new Set(
                      steps.flatMap(
                        (step) =>
                          step.effects?.map(
                            (effect) => effect.effect ?? "unknown",
                          ) ?? [],
                      ),
                    ),
                  ].join(" + ") || "none"}
                </strong>
                <small>
                  {run.kind.startsWith("safe_")
                    ? "safe mode accepted"
                    : "standard mode"}
                </small>
              </div>
            </div>
            <div class="execution-review">
              <h2>Execution</h2>
              <ol>
                {visibleSteps.map((step) => (
                  <li key={step.name}>
                    <span class={`dot ${step.status}`} />
                    <strong>{step.name}</strong>
                    <small>
                      {step.effects
                        ?.map((effect) => effect.effect ?? "unknown")
                        .join(" · ")}
                    </small>
                    <time>{step.duration_ms} ms</time>
                  </li>
                ))}
                {!steps.length && <li class="empty">No steps reported.</li>}
              </ol>
              {passedSteps.length > 0 && (
                <details class="passed-steps">
                  <summary>
                    {passedSteps.length} passed{" "}
                    {passedSteps.length === 1 ? "step" : "steps"}
                  </summary>
                  <ol>
                    {passedSteps.map((step) => (
                      <li key={step.name}>
                        <span class={`dot ${step.status}`} />
                        <strong>{step.name}</strong>
                        <small>
                          {step.effects
                            ?.map((effect) => effect.effect ?? "unknown")
                            .join(" · ")}
                        </small>
                        <time>{step.duration_ms} ms</time>
                      </li>
                    ))}
                  </ol>
                </details>
              )}
            </div>
          </section>

          <section
            id="panel-diagnostics"
            class="review-panel"
            role="tabpanel"
            aria-labelledby="tab-diagnostics"
            hidden={activeTab !== "diagnostics"}
          >
            <div class="panel-title">
              <div>
                <p class="eyebrow">
                  {diagnostics.length} issues in {diagnosticsByFile.size} files
                </p>
                <h2>Diagnostics</h2>
              </div>
            </div>
            <div class="diagnostic-list">
              {[...diagnosticsByFile].map(([file, items]) =>
                items.map((item, index) => (
                  <article key={`${file}-${item.rule}-${index}`}>
                    <b class={item.severity}>
                      {item.severity.slice(0, 1).toUpperCase()}
                    </b>
                    <span class="location">
                      <strong>{file}</strong>
                      <small>
                        {item.range?.start.line
                          ? `line ${item.range.start.line}`
                          : "project"}{" "}
                        · {item.rule || item.tool}
                      </small>
                    </span>
                    <span class="message">{item.message}</span>
                  </article>
                )),
              )}
              {!diagnostics.length && (
                <p class="empty">No normalized diagnostics.</p>
              )}
            </div>
          </section>

          <section
            id="panel-logs"
            class="review-panel"
            role="tabpanel"
            aria-labelledby="tab-logs"
            hidden={activeTab !== "logs"}
          >
            <div class="panel-title">
              <div>
                <p class="eyebrow">Captured output</p>
                <h2>Logs</h2>
              </div>
              <input
                aria-label="Search logs"
                placeholder="Filter logs"
                value={query}
                onInput={(event) => setQuery(event.currentTarget.value)}
              />
            </div>
            <pre tabIndex={0}>{visibleLogs || "No output captured."}</pre>
          </section>

          <section
            id="panel-patch"
            class="review-panel patch-panel"
            role="tabpanel"
            aria-labelledby="tab-patch"
            hidden={activeTab !== "patch"}
          >
            <div class="panel-title">
              <div>
                <p class="eyebrow">Working tree</p>
                <h2>Patch</h2>
              </div>
            </div>
            <pre tabIndex={0}>
              {diffLoading
                ? "Loading patch…"
                : diffError
                  ? "Patch unavailable."
                  : diff || "Working tree unchanged."}
            </pre>
          </section>

          <footer class="review-footer">
            <span>
              <i /> Structured result · schema v1
            </span>
            <span>Updated just now</span>
          </footer>
        </>
      )}
    </main>
  );
}
