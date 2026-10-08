#!/usr/bin/env python3
"""Karakuri All-in-One Dev Server with Log Relay.

Serves the built web app from `dist/` and receives `/log` POST requests
on the SAME port, eliminating CORS and Secure Context multi-origin issues.

Usage:
    python3 tools/remote_log_server.py [port]
    (default port is 8080)
"""

from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import os
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
DIST_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "dist"))

class DevServerHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIST_DIR, **kwargs)

    def end_headers(self):
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")
        super().end_headers()

    def do_POST(self):
        if self.path == "/log":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length).decode("utf-8", errors="replace")
            sys.stdout.write(f"\033[36m[Quest]\033[0m {body}\n")
            sys.stdout.flush()

            try:
                os.makedirs("target", exist_ok=True)
                with open("target/quest_remote.log", "a", encoding="utf-8") as f:
                    f.write(f"[Quest] {body}\n")
            except Exception:
                pass

            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"ok")
        else:
            self.send_error(404, "Not Found")

    def log_message(self, format, *args):
        # Silence standard HTTP static file GET logs to keep terminal focused on Quest logs
        if self.command == "POST":
            return
        # Uncomment below if you want to see static file requests:
        # super().log_message(format, *args)

if __name__ == "__main__":
    if not os.path.isdir(DIST_DIR):
        print(f"\033[31mError: dist directory not found at {DIST_DIR}\033[0m")
        print("Please run `trunk build` first.")
        sys.exit(1)

    server = ThreadingHTTPServer(("0.0.0.0", PORT), DevServerHandler)
    print(f"\033[32m=== Karakuri Dev Server & Log Relay running on http://0.0.0.0:{PORT} ===\033[0m")
    print(f"Serving web app from: {DIST_DIR}")
    print(f"Log endpoint: http://0.0.0.0:{PORT}/log (Same-Origin)")
    print("Waiting for Quest Browser connection... (Ctrl+C to stop)\n")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopped.")
