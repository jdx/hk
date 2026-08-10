import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/preact";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Dashboard } from "./app";
import type { RunSnapshot } from "./types";

const run: RunSnapshot = {
  id: "hk-1",
  root: "/demo",
  kind: "safe_check",
  status: "failed",
  started_at: "2026-01-01T00:00:00Z",
  output_bytes: 10,
  has_diff: true,
  result: {
    status: "failed",
    duration_ms: 37,
    steps: [
      {
        name: "compiler",
        status: "failed",
        duration_ms: 12,
        diagnostics: [
          {
            step: "compiler",
            tool: "cc",
            severity: "warning",
            message: "unused value",
            path: "src/main.c",
            rule: "W1",
          },
        ],
      },
    ],
  },
};

afterEach(cleanup);

describe("Dashboard", () => {
  it("uses the live execution layout while a run is active", async () => {
    const active: RunSnapshot = {
      ...run,
      status: "running",
      has_diff: false,
      result: {
        status: "running",
        duration_ms: 0,
        steps: [
          {
            name: "compiler",
            status: "running",
            duration_ms: 0,
            effects: [{ command: "check", effect: "read" }],
            diagnostics: [],
          },
        ],
      },
    };
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return active;
      if (name === "get_output")
        return { text: "checking compiler", eof: true, next_offset: 17 };
      return active;
    });

    render(<Dashboard initial={active} call={call} />);
    expect(screen.getByRole("img", { name: "hk" })).toBeInTheDocument();
    expect(screen.getByText("0 of 1 steps complete")).toBeInTheDocument();
    expect(await screen.findByText("checking compiler")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Cancel run" })).toBeEnabled();
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Confirm safe fix" }),
    ).not.toBeInTheDocument();
  });

  it("switches completed run review tabs", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return run;
      if (name === "get_output")
        return { text: "compiler output", eof: true, next_offset: 15 };
      if (name === "get_diff")
        return { text: "+fixed", eof: true, next_offset: 6 };
      return run;
    });

    render(<Dashboard initial={run} call={call} />);
    const diagnosticsTab = screen.getByRole("tab", {
      name: /diagnostics/i,
    });
    expect(diagnosticsTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByRole("tabpanel", { name: /diagnostics/i }),
    ).toBeVisible();

    fireEvent.click(screen.getByRole("tab", { name: /logs/i }));
    expect(screen.getByRole("tabpanel", { name: /logs/i })).toBeVisible();
    expect(await screen.findByText("compiler output")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /patch/i }));
    expect(screen.getByRole("tabpanel", { name: /patch/i })).toBeVisible();
    expect(await screen.findByText("+fixed")).toBeInTheDocument();
  });

  it("supports arrow-key navigation across review tabs", () => {
    const call = vi.fn(async () => run);
    render(<Dashboard initial={run} call={call} />);
    const diagnosticsTab = screen.getByRole("tab", {
      name: /diagnostics/i,
    });

    diagnosticsTab.focus();
    fireEvent.keyDown(diagnosticsTab, { key: "ArrowRight" });

    expect(screen.getByRole("tab", { name: /logs/i })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("renders diagnostics, filters logs, and exposes safe actions", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return run;
      if (name === "get_output")
        return { text: "first line\nwarning line", eof: true, next_offset: 23 };
      if (name === "get_diff")
        return { text: "+fixed", eof: true, next_offset: 6 };
      return run;
    });
    render(<Dashboard initial={run} call={call} />);

    expect(await screen.findByText("src/main.c")).toBeInTheDocument();
    expect(screen.getByText("unused value")).toBeInTheDocument();
    expect(screen.getByText("37 ms")).toBeInTheDocument();
    fireEvent.input(screen.getByLabelText("Search logs"), {
      target: { value: "warning" },
    });
    expect(await screen.findByText(/warning line/)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Confirm safe fix" }),
    ).toBeEnabled();
  });

  it("does not claim normal checks used safe mode", () => {
    const normal = { ...run, kind: "check" };
    render(<Dashboard initial={normal} call={vi.fn(async () => normal)} />);

    expect(screen.getByText("standard mode")).toBeInTheDocument();
    expect(screen.queryByText("safe mode accepted")).not.toBeInTheDocument();
  });

  it("collapses passed steps and keeps non-passing steps visible", () => {
    const mixed: RunSnapshot = {
      ...run,
      has_diff: false,
      result: {
        ...run.result!,
        steps: [
          {
            name: "format",
            status: "passed",
            duration_ms: 4,
            diagnostics: [],
          },
          {
            name: "lint",
            status: "failed",
            duration_ms: 8,
            diagnostics: [],
          },
        ],
      },
    };

    render(<Dashboard initial={mixed} call={vi.fn(async () => mixed)} />);

    const disclosure = screen.getByText("1 passed step").closest("details");
    expect(disclosure).not.toHaveAttribute("open");
    expect(screen.getByText("lint")).toBeVisible();
  });

  it("links pre-step failures to captured logs", async () => {
    const setupFailure: RunSnapshot = {
      ...run,
      result: undefined,
      has_diff: false,
    };
    const call = vi.fn(async (name: string) => {
      if (name === "get_output")
        return { text: "configuration failed", eof: true, next_offset: 20 };
      return setupFailure;
    });

    render(<Dashboard initial={setupFailure} call={call} />);
    expect(
      screen.getByText("Run failed before step results were available"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View logs" }));
    expect(await screen.findByText("configuration failed")).toBeVisible();
  });

  it("replaces state when a different run is rendered", async () => {
    const next = { ...run, id: "hk-2", kind: "safe_fix", has_diff: false };
    const call = vi.fn(async (name: string, args: Record<string, unknown>) => {
      const selected = args.run_id === "hk-2" ? next : run;
      if (name === "get_run") return selected;
      if (name === "get_output")
        return { text: selected.id, eof: true, next_offset: 4 };
      if (name === "get_diff")
        return { text: "+old", eof: true, next_offset: 4 };
      return selected;
    });
    const view = render(<Dashboard initial={run} call={call} />);
    expect(await screen.findByText("+old")).toBeInTheDocument();
    fireEvent.input(screen.getByLabelText("Search logs"), {
      target: { value: "old filter" },
    });

    view.rerender(<Dashboard initial={next} call={call} />);
    expect(await screen.findByText("safe fix")).toBeInTheDocument();
    expect(
      await screen.findByText("Working tree unchanged."),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Search logs")).toHaveValue("");
    expect(screen.queryByText("+old")).not.toBeInTheDocument();
  });

  it("shows a loading state until the advertised patch arrives", async () => {
    let resolveDiff!: (value: {
      text: string;
      eof: boolean;
      next_offset: number;
    }) => void;
    const pendingDiff = new Promise<{
      text: string;
      eof: boolean;
      next_offset: number;
    }>((resolve) => {
      resolveDiff = resolve;
    });
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return run;
      if (name === "get_output")
        return { text: "log", eof: true, next_offset: 3 };
      if (name === "get_diff") return pendingDiff;
      return run;
    });

    render(<Dashboard initial={run} call={call} />);
    expect(screen.getByText("Loading patch…")).toBeInTheDocument();
    expect(
      screen.queryByText("Working tree unchanged."),
    ).not.toBeInTheDocument();
    resolveDiff({ text: "+fixed", eof: true, next_offset: 6 });
    expect(await screen.findByText("+fixed")).toBeInTheDocument();
  });

  it("clears the patch loading state when patch retrieval fails", async () => {
    const refreshed = { ...run, kind: "safe_fix" };
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return refreshed;
      if (name === "get_output")
        return { text: "log", eof: true, next_offset: 3 };
      if (name === "get_diff") throw new Error("patch unavailable");
      return run;
    });

    render(<Dashboard initial={run} call={call} />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "patch unavailable",
    );
    expect(screen.queryByText("Loading patch…")).not.toBeInTheDocument();
    expect(screen.getByText("Patch unavailable.")).toBeInTheDocument();
    expect(
      screen.queryByText("Working tree unchanged."),
    ).not.toBeInTheDocument();
    expect(screen.getByText("safe fix")).toBeInTheDocument();
  });

  it("does not relabel the patch when log retrieval fails", async () => {
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return run;
      if (name === "get_output") throw new Error("logs unavailable");
      return run;
    });

    render(<Dashboard initial={run} call={call} />);
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "logs unavailable",
    );
    expect(screen.getByText("Loading patch…")).toBeInTheDocument();
    expect(screen.queryByText("Patch unavailable.")).not.toBeInTheDocument();
  });

  it("shows action failures and follows output pagination", async () => {
    let outputPage = 0;
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") return run;
      if (name === "get_output") {
        outputPage += 1;
        return outputPage === 1
          ? { text: "page one\n", eof: false, next_offset: 9, total_bytes: 17 }
          : { text: "page two", eof: true, next_offset: 17, total_bytes: 17 };
      }
      if (name === "get_diff") return { text: "", eof: true, next_offset: 0 };
      if (name === "start_safe_check") throw new Error("safe run rejected");
      return run;
    });
    render(<Dashboard initial={{ ...run, has_diff: false }} call={call} />);
    expect(await screen.findByText(/page one/)).toBeInTheDocument();
    expect(screen.getByText(/page two/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Rerun safe check" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "safe run rejected",
    );
  });

  it("keeps an action failure visible when a concurrent refresh succeeds", async () => {
    let resolveRefresh!: (value: RunSnapshot) => void;
    const pendingRefresh = new Promise<RunSnapshot>((resolve) => {
      resolveRefresh = resolve;
    });
    let getRunCount = 0;
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") {
        getRunCount += 1;
        return getRunCount === 1 ? run : pendingRefresh;
      }
      if (name === "get_output")
        return { text: "log", eof: true, next_offset: 3 };
      if (name === "get_diff")
        return { text: "+fixed", eof: true, next_offset: 6 };
      if (name === "start_safe_check") throw new Error("safe run rejected");
      return run;
    });

    render(<Dashboard initial={run} call={call} />);
    await waitFor(() => expect(getRunCount).toBe(1));
    fireEvent.click(screen.getByRole("button", { name: "Rerun safe check" }));
    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "safe run rejected",
    );
    resolveRefresh(run);
    await waitFor(() => expect(getRunCount).toBe(2));
    expect(screen.getByRole("alert")).toHaveTextContent("safe run rejected");
  });

  it("does not let an older refresh overwrite a newer response", async () => {
    let getRunCount = 0;
    let resolveOlder!: (value: RunSnapshot) => void;
    let resolveNewer!: (value: RunSnapshot) => void;
    const older = new Promise<RunSnapshot>((resolve) => {
      resolveOlder = resolve;
    });
    const newer = new Promise<RunSnapshot>((resolve) => {
      resolveNewer = resolve;
    });
    const call = vi.fn(async (name: string) => {
      if (name === "get_run") {
        getRunCount += 1;
        if (getRunCount === 2) return older;
        if (getRunCount === 3) return newer;
        return run;
      }
      if (name === "get_output")
        return { text: "log", eof: true, next_offset: 3 };
      if (name === "get_diff") return { text: "", eof: true, next_offset: 0 };
      return run;
    });
    render(<Dashboard initial={{ ...run, has_diff: false }} call={call} />);
    await waitFor(() => expect(getRunCount).toBe(1));

    const refresh = screen.getByRole("button", { name: "Refresh" });
    fireEvent.click(refresh);
    fireEvent.click(refresh);
    await waitFor(() => expect(getRunCount).toBe(3));
    resolveNewer({ ...run, kind: "safe_fix", has_diff: false });
    expect(await screen.findByText("safe fix")).toBeInTheDocument();
    resolveOlder({ ...run, kind: "check", has_diff: false });
    await Promise.resolve();

    expect(screen.getByText("safe fix")).toBeInTheDocument();
  });
});
