"""Serve the built static game at the same /framesmith/ prefix as GitHub Pages."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class Handler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            self.send_response(302)
            self.send_header('Location', '/framesmith/')
            self.end_headers()
            return
        if not self.path.startswith('/framesmith/'):
            self.send_error(404)
            return
        self.path = self.path[len('/framesmith'):]
        super().do_GET()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port', type=int, default=8080)
    args = parser.parse_args()
    handler = partial(Handler, directory=str(Path(__file__).resolve().parent / 'dist'))
    with ThreadingHTTPServer(('127.0.0.1', args.port), handler) as server:
        print(f'FrameSmith Arena: http://127.0.0.1:{args.port}/framesmith/', flush=True)
        server.serve_forever()
