import { App } from "@modelcontextprotocol/ext-apps";

export type ToolCaller = (
  name: string,
  args: Record<string, unknown>,
) => Promise<unknown>;

export function structuredContent(result: unknown): unknown {
  if (!result || typeof result !== "object") return undefined;
  return (result as { structuredContent?: unknown }).structuredContent;
}

export async function connectBridge(onResult: (value: unknown) => void) {
  const app = new App({ name: "hk dashboard", version: "1" });
  app.ontoolresult = (result) => onResult(structuredContent(result));
  await app.connect();
  const call: ToolCaller = async (name, args) =>
    structuredContent(await app.callServerTool({ name, arguments: args }));
  return { app, call };
}
