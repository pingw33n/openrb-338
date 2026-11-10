#!/usr/bin/python3
"""A minimal MCP client over stdio: initialize, list the tools and call one.

Usage: mcp_smoke.py BIN [TOOL [JSON_ARGS]]

The transport is the one MCP clients use: one JSON-RPC message per line on the server's stdin,
its answers read from stdout. A message that is not a response (a notification or a request) is
skipped.
"""
import json
import subprocess
import sys


class Client:
    def __init__(self, binary):
        self.p = subprocess.Popen(
            [binary],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.next_id = 0

    def send(self, method, params=None, notify=False):
        msg = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        if not notify:
            self.next_id += 1
            msg["id"] = self.next_id
        self.p.stdin.write(json.dumps(msg) + "\n")
        self.p.stdin.flush()
        if notify:
            return None
        while True:
            line = self.p.stdout.readline()
            if not line:
                err = self.p.stderr.read()
                raise RuntimeError(f"the server closed the connection: {err}")
            try:
                reply = json.loads(line)
            except json.JSONDecodeError:
                continue
            if reply.get("id") == msg["id"]:
                return reply

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=10)
        return self.p.stderr.read()


def main(argv):
    binary = argv[1]
    c = Client(binary)
    init = c.send(
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "orb-mcp-smoke", "version": "0"},
        },
    )
    server = init["result"]["serverInfo"]
    print(f"server: {server['name']} {server['version']}")
    c.send("notifications/initialized", {}, notify=True)

    if len(argv) < 3:
        tools = c.send("tools/list")["result"]["tools"]
        print(f"tools: {len(tools)}")
        for t in tools:
            props = ", ".join(sorted((t.get("inputSchema") or {}).get("properties", {})))
            print(f"  {t['name']}({props})")
        c.close()
        return 0

    args = json.loads(argv[3]) if len(argv) > 3 else {}
    reply = c.send("tools/call", {"name": argv[2], "arguments": args})
    if "error" in reply:
        print(f"protocol error: {reply['error']}", file=sys.stderr)
        c.close()
        return 1
    res = reply["result"]
    for block in res.get("content", []):
        print(block.get("text", ""))
    c.close()
    return 0 if not res.get("isError") else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
