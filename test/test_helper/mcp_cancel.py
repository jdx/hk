#!/usr/bin/env python3
"""Start a run through a real `hk mcp` over stdio, cancel it, and check cleanup.

usage: mcp_cancel.py TOKEN   (run from the project directory; `hk` is on PATH)

The project's step must create ./started once it is running and carry TOKEN in
its command line, so leftover processes can be found.
"""
import json
import os
import subprocess
import sys
import time

token = sys.argv[1]
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


def leftovers():
    out = subprocess.run(
        ["ps", "-eo", "pid,args"], capture_output=True, text=True, check=True
    ).stdout
    return [
        line.strip()
        for line in out.splitlines()
        if token in line and "mcp_cancel.py" not in line
    ]


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
    run_id = tool("start_check")["id"]

    deadline = time.time() + 30
    while not os.path.exists("started"):
        assert time.time() < deadline, "step never started"
        time.sleep(0.05)

    tool("cancel_run", {"run_id": run_id})
    started = time.time()
    status = "cancelling"
    while status == "cancelling" and time.time() - started < 8:
        status = tool("get_run", {"run_id": run_id})["status"]
        time.sleep(0.1)
    print(f"status={status} after {time.time() - started:.1f}s")
    assert status == "cancelled", f"run is {status}, expected cancelled"

    time.sleep(0.3)
    left = leftovers()
    assert not left, "processes survived cancel_run:\n" + "\n".join(left)
finally:
    server.kill()
    server.wait()
