#!/usr/bin/env python3
# share_parallel.py: the share server under the load a browser makes. Many requests at once, some
# from connections that send their request late (as over Wi-Fi), one connection that never sends
# anything (browsers open spares) and one that hangs up while queued: every answer must be the
# one its own request asked for, byte for byte, and nothing may be refused as busy.
#   usage: python3 -I tests/share_parallel.py <port> <notebook id> <pages>
import socket
import sys
import threading
import time

PORT, NB, PAGES = int(sys.argv[1]), sys.argv[2], int(sys.argv[3])


def get(path, delay=0.0, timeout=60):
    s = socket.create_connection(("127.0.0.1", PORT), timeout=timeout)
    if delay:
        time.sleep(delay)
    s.sendall(f"GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".encode())
    data = b""
    while True:
        b = s.recv(1 << 16)
        if not b:
            break
        data += b
    s.close()
    head, _, body = data.partition(b"\r\n\r\n")
    return head.split(b"\r\n")[0].decode(errors="replace"), body


paths = ["/api/list.json", "/"]
for n in range(1, PAGES + 1):
    paths += [f"/nb/{NB}/{n}.png?layer=ink", f"/nb/{NB}/{n}.png?layer=paper&f=2", f"/nb/{NB}/{n}.png?f=3"]
# What each path answers when asked alone.
ref = {p: get(p)[1] for p in paths}

# A connection that never says anything, and one that hangs up while its render waits.
idle = socket.create_connection(("127.0.0.1", PORT))
quitter = socket.create_connection(("127.0.0.1", PORT))
quitter.sendall(f"GET /nb/{NB}/1.png HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".encode())
quitter.close()

results, threads = {}, []
t0 = time.time()
for i, p in enumerate(paths * 2):
    def run(i=i, p=p):
        try:
            results[i] = (p,) + get(p, delay=0.25 if i % 3 == 0 else 0.0)
        except Exception as e:  # noqa: BLE001
            results[i] = (p, f"error {e}", b"")
    t = threading.Thread(target=run)
    t.start()
    threads.append(t)
for t in threads:
    t.join()
took = time.time() - t0
idle.close()

bad = 0
for i in sorted(results):
    p, status, body = results[i]
    if status != "HTTP/1.0 200 OK" or body != ref[p]:
        bad += 1
        print(f"FAIL {p}: {status}, {len(body)} bytes (alone: {len(ref[p])})")
print(f"{len(results) - bad}/{len(results)} parallel answers right in {took:.1f} s")
sys.exit(1 if bad or len(results) != len(paths) * 2 else 0)
