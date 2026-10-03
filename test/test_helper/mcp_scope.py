#!/usr/bin/env python3
"""Run `start_check` with a scope through a real `hk mcp` over stdio and wait for it.

usage: mcp_scope.py SCOPE   (run from the project directory; `hk` is on PATH)

Prints `status=<final status>` once the run is no longer active.
"""
import json
import subprocess
import sys
import time

scope = sys.argv[1]
server = subprocess.Popen(
    ["hk", "mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
    text=True,
)
next_id = 0


def rpc(method, params=None, notify=False):
    global next_id
    message = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    if not notify:
        next_id += 1
        message["id"] = next_id
    server.stdin.write(json.dumps(message) + "\n")
    server.stdin.flush()
    while not notify:
        response = json.loads(server.stdout.readline())
        if response.get("id") == next_id:
            return response


def tool(name, arguments=None):
    response = rpc("tools/call", {"name": name, "arguments": arguments or {}})
    return response["result"]["structuredContent"]


try:
    rpc(
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "hk-test", "version": "1"},
        },
    )
    rpc("notifications/initialized", notify=True)
    run_id = tool("start_check", {"scope": scope})["id"]
    deadline = time.time() + 60
    status = "running"
    while status in ("starting", "running", "cancelling"):
        assert time.time() < deadline, "run never finished"
        time.sleep(0.1)
        status = tool("get_run", {"run_id": run_id})["status"]
    print(f"status={status}")
finally:
    server.kill()
    server.wait()
