import { render } from "preact";
import { Dashboard } from "./app";
import { connectBridge } from "./bridge";
import type { ToolCaller } from "./bridge";
import type { RunSnapshot } from "./types";
import "./style.css";

const root = document.getElementById("app")!;
let bridge: Awaited<ReturnType<typeof connectBridge>> | undefined;
let pendingRun: RunSnapshot | undefined;
const call: ToolCaller = (name, args) =>
  bridge
    ? bridge.call(name, args)
    : Promise.reject(new Error("MCP Apps bridge is not ready"));

const renderRun = (run: RunSnapshot) =>
  render(<Dashboard key={run.id} initial={run} call={call} />, root);

connectBridge((value) => {
  const payload = value as { run?: RunSnapshot } | undefined;
  if (!payload?.run) return;
  if (bridge) renderRun(payload.run);
  else pendingRun = payload.run;
})
  .then((connected) => {
    bridge = connected;
    if (pendingRun) {
      renderRun(pendingRun);
      pendingRun = undefined;
    } else if (!root.hasChildNodes()) {
      root.innerHTML =
        '<div class="boot">Connected. Select an hk run to render.</div>';
    }
  })
  .catch(() => {
    root.innerHTML =
      '<main class="fallback"><p class="eyebrow">hk dashboard</p><h1>Structured run view</h1><p>This host does not expose the MCP Apps bridge. The same run remains available through structured tool results.</p></main>';
  });
