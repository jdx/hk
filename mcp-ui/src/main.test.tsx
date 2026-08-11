import { waitFor } from "@testing-library/preact";
import { beforeEach, expect, test, vi } from "vitest";

const bridgeState = vi.hoisted(() => ({
  listener: undefined as ((value: unknown) => void) | undefined,
}));

vi.mock("./app", () => ({
  Dashboard: () => <main>Rendered run</main>,
}));

vi.mock("./bridge", () => ({
  connectBridge: vi.fn((listener: (value: unknown) => void) => {
    bridgeState.listener = listener;
    return Promise.resolve({ call: vi.fn() });
  }),
}));

beforeEach(() => {
  vi.resetModules();
  bridgeState.listener = undefined;
  document.body.innerHTML = '<div id="app"></div>';
});

test("replaces the connected placeholder when a run arrives", async () => {
  await import("./main");

  await waitFor(() =>
    expect(document.body).toHaveTextContent(
      "Connected. Select an hk run to render.",
    ),
  );

  bridgeState.listener?.({ run: { id: "hk-1" } });

  await waitFor(() => expect(document.body).toHaveTextContent("Rendered run"));
  expect(document.body).not.toHaveTextContent(
    "Connected. Select an hk run to render.",
  );
});
