#!/usr/bin/env python3
"""Run `start_check` with a scope through a real `hk mcp` over stdio and wait for it.

usage: mcp_scope.py SCOPE   (run from the project directory; `hk` is on PATH)

Prints `status=<final status>` once the run is no longer active.
"""
import json
import os
import queue
import subprocess
import sys
import threading
import time

REPLY_TIMEOUT = float(os.environ.get("MCP_SCOPE_REPLY_TIMEOUT", "30"))

scope = sys.argv[1]
server = subprocess.Popen(
    ["hk", "mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.DEVNULL,
    text=True,
    # Own session/process group so cleanup can signal everything it spawned.
    # These bats run on unix CI only; on Windows the child is just killed.
    start_new_session=(os.name == "posix"),
)
next_id = 0
run_id = None

# Read stdout on a thread so a reply wait can time out (works on Windows too,
# where select() does not support pipes). A silent server fails the test
# instead of hanging it; the `finally` below kills the child.
lines = queue.Queue()


def pump():
    for line in server.stdout:
        lines.put(line)
    lines.put(None)


threading.Thread(target=pump, daemon=True).start()


def read_reply(timeout=REPLY_TIMEOUT):
    try:
        line = lines.get(timeout=timeout)
    except queue.Empty:
        raise TimeoutError(f"no reply from hk mcp within {timeout}s")
    if line is None:
        raise EOFError("hk mcp closed stdout")
    return json.loads(line)


def rpc(method, params=None, notify=False, timeout=REPLY_TIMEOUT):
    global next_id
    message = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    if not notify:
        next_id += 1
        message["id"] = next_id
    server.stdin.write(json.dumps(message) + "\n")
    server.stdin.flush()
    while not notify:
        response = read_reply(timeout)
        if response.get("id") == next_id:
            return response


def tool(name, arguments=None, timeout=REPLY_TIMEOUT):
    response = rpc(
        "tools/call", {"name": name, "arguments": arguments or {}}, timeout=timeout
    )
    return response["result"]["structuredContent"]


def descendants(pid):
    """All transitive children of pid (the `hk check` run is in its own group)."""
    try:
        out = subprocess.run(
            ["ps", "-A", "-o", "pid=,ppid="], capture_output=True, text=True, timeout=5
        ).stdout
    except Exception:
        return []
    children = {}
    for row in out.splitlines():
        parts = row.split()
        if len(parts) == 2:
            children.setdefault(int(parts[1]), []).append(int(parts[0]))
    found, stack = [], [pid]
    while stack:
        for child in children.get(stack.pop(), []):
            found.append(child)
            stack.append(child)
    return found


def cleanup():
    """Stop the active run, then the server, then anything left over."""
    if run_id is not None and server.poll() is None:
        try:
            tool("cancel_run", {"run_id": run_id}, timeout=5)
            end = time.time() + 5
            while time.time() < end:
                if tool("get_run", {"run_id": run_id}, timeout=5)["status"] not in (
                    "starting",
                    "running",
                    "cancelling",
                ):
                    break
                time.sleep(0.1)
        except Exception:
            pass  # best effort: fall through to the hard kill
    leftovers = descendants(server.pid) if os.name == "posix" else []
    if server.poll() is None:
        server.terminate()
        try:
            server.wait(timeout=3)
        except subprocess.TimeoutExpired:
            server.kill()
    if os.name == "posix":
        import signal

        try:
            os.killpg(server.pid, signal.SIGKILL)
        except OSError:
            pass
        for pid in leftovers:
            try:
                os.kill(pid, signal.SIGKILL)
            except OSError:
                pass
    else:
        subprocess.run(
            ["taskkill", "/T", "/F", "/PID", str(server.pid)], capture_output=True
        )
    server.kill()
    server.wait()


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
    cleanup()
