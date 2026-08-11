# hk MCP dashboard

`hk mcp` embeds a self-contained [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview) dashboard. Compatible hosts show live run state, normalized diagnostics, effects, logs, and the resulting Git patch. Other clients receive the same authoritative structured content and text summaries.

## Local demo fixture

```bash
fixture=$(./scripts/create_mcp_demo)
hk mcp --root "$fixture"
```

Connect that STDIO command through the host. In ChatGPT Desktop, use OpenAI's [Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels). Start a safe check and call `render_run` with its run ID. The fixture pauses briefly so the live execution layout is visible, then reports the `demo.txt` diagnostic in the completed review layout. Use **Fix safely** to generate the resulting patch.

The production server remains STDIO-only; it opens no port and hosts no service.
