#!/usr/bin/env python3
"""A stand-in for the Bluetooth helper's local API (kindle-hid-passthrough 3.18's, as far as Grimoire
asks it), for tests/bt_check.sh: /health, /status, /scan and /scan-status (two controllers found
after a moment, one with a name that is not printable), /pair and /pair-status (pairs the first,
refuses the second), /remove. Every request is logged to stdout, one line each. With a file named, it
writes the port it listens on there once it does (port 0: any free one).
usage: fake_bthelper.py <port> [port-file]"""
import json
import os
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import TCPServer
from urllib.parse import parse_qs, urlparse

FOUND = [
    {"address": "02:00:5E:10:20:30", "name": "Pro Controller", "protocol": "classic", "rssi": -48},
    {"address": "02:00:5E:10:20:31", "name": "Stick\u0007 é", "protocol": "ble", "rssi": -70},
]
state = {"paired": [], "scan_at": None, "pair": None}
lock = threading.Lock()


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def send(self, data):
        body = json.dumps(data).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        u = urlparse(self.path)
        q = {k: v[0] for k, v in parse_qs(u.query).items()}
        print('GET', self.path, flush=True)
        now = time.time()
        with lock:
            if u.path == '/health':
                return self.send({"ok": True})
            if u.path == '/status':
                return self.send({"ok": True, "version": "3.18.0-test", "daemon_running": True,
                                  "scanning": state["scan_at"] is not None and now - state["scan_at"] < 1.0,
                                  "pairing": False,
                                  "devices": [{"address": d["address"], "protocol": d["protocol"], "name": d["name"]}
                                              for d in state["paired"]],
                                  "connections": [{"address": d["address"], "protocol": d["protocol"],
                                                   "name": d["name"], "hid_ready": True,
                                                   "input_paths": ["/dev/input/event6"]} for d in state["paired"]]})
            if u.path == '/scan':
                state["scan_at"] = now
                return self.send({"ok": True, "message": "Scan started"})
            if u.path == '/scan-status':
                if state["scan_at"] is None:
                    return self.send({"ok": False, "error": "No scan in progress"})
                if now - state["scan_at"] < 1.0:
                    return self.send({"ok": True, "scanning": True, "devices": FOUND[:1]})
                return self.send({"ok": True, "devices": FOUND})
            if u.path == '/pair':
                addr = q.get("addr", "")
                ok = addr == FOUND[0]["address"]
                state["pair"] = (now, addr, ok)
                return self.send({"ok": True, "message": "Pairing started"})
            if u.path == '/pair-status':
                if not state["pair"]:
                    return self.send({"ok": False, "error": "No pairing in progress"})
                t, addr, ok = state["pair"]
                if now - t < 0.8:
                    return self.send({"ok": True, "pairing": True})
                if ok and not any(d["address"] == addr for d in state["paired"]):
                    state["paired"].append(dict(FOUND[0]))
                return self.send({"ok": ok, "address": addr,
                                  **({"message": "Paired successfully"} if ok else {"error": "Pairing failed"})})
            if u.path == '/remove':
                addr = q.get("addr", "")
                before = len(state["paired"])
                state["paired"] = [d for d in state["paired"] if d["address"] != addr]
                if len(state["paired"]) < before:
                    return self.send({"ok": True, "message": "Device removed", "address": addr})
                return self.send({"ok": False, "error": "Device not found: " + addr})
        self.send({"ok": False, "error": "Not found"})


class Server(HTTPServer):
    def server_bind(self):
        # (HTTPServer's own looks its address up in DNS between bind and listen: on macOS a connection
        # made in between is not refused but left waiting.)
        TCPServer.server_bind(self)
        self.server_name, self.server_port = '127.0.0.1', self.server_address[1]


if __name__ == '__main__':
    srv = Server(('127.0.0.1', int(sys.argv[1])), H)
    if len(sys.argv) > 2:
        with open(sys.argv[2] + '.tmp', 'w') as f:
            f.write('%d\n' % srv.server_address[1])
        os.replace(sys.argv[2] + '.tmp', sys.argv[2])
    srv.serve_forever()
