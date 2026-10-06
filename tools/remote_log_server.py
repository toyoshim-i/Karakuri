#!/usr/bin/env python3
"""Karakuri Remote Log Relay Server.

Listens on 0.0.0.0:8081 and prints incoming logs from Quest Browser in real-time.
Usage:
    python3 tools/remote_log_server.py
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import sys

class LogHandler(BaseHTTPRequestHandler):
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'POST, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()

    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(length).decode('utf-8', errors='replace')
        sys.stdout.write(f"\033[36m[Quest]\033[0m {body}\n")
        sys.stdout.flush()

        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(b'ok')

    def log_message(self, format, *args):
        # Silence default HTTP access log lines to keep terminal clean
        pass

if __name__ == '__main__':
    port = 8081
    server = HTTPServer(('0.0.0.0', port), LogHandler)
    print(f"\033[32m=== Karakuri Quest Log Relay running on http://0.0.0.0:{port} ===\033[0m")
    print("Waiting for logs from Quest Browser... (Ctrl+C to stop)\n")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopped.")
