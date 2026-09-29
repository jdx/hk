---
sourceHash: 320226173211
---

# The hk MCP dashboard, a view from the quarterdeck

`hk mcp` embeds a self-contained [MCP Apps](https://modelcontextprotocol.io/extensions/apps/overview) dashboard, stowed aboard and ready to hoist. Compatible hosts show the live state of the run, normalized diagnostics, effects, logs, and the resulting Git patch. Other clients get the same authoritative structured content, plus text summaries.

## A practice ship: the local demo fixture {#local-demo-fixture}

```bash
fixture=$(./scripts/create_mcp_demo)
hk mcp --root "$fixture"
```

Connect that STDIO command through yer host. In ChatGPT Desktop, use OpenAI's [Secure MCP Tunnel](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels). Start a safe check and call `render_run` with its run ID. The fixture heaves to briefly, pausing so ye can see the live execution layout, then reports the `demo.txt` diagnostic in the completed review layout. Use **Fix safely** to generate the resulting patch.

Mark this, sailor: the production server stays STDIO-only. It opens no port and hosts no service.
