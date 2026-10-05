#!/usr/bin/env python3
"""Build web_landing for the browser and serve it on http://localhost:8080 (no Trunk needed).

    python examples/web_landing/serve.py            # build, then serve
    python examples/web_landing/serve.py --no-build # serve the last build
    python examples/web_landing/serve.py --port 9000

Needs the wasm32 target and a wasm-bindgen CLI equal to the wasm-bindgen in Cargo.lock.
Add ?zaxis_backend=webgl2 to the URL to force the WebGL2 fallback.
"""
import argparse
import functools
import http.server
import pathlib
import shutil
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DIST = ROOT / "target" / "web_landing" / "dist"
WASM = ROOT / "target" / "wasm32-unknown-unknown" / "release" / "examples" / "web_landing.wasm"


def run(*command):
    print("+", " ".join(map(str, command)), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)


def build():
    run("cargo", "build", "--release", "--example", "web_landing", "--features", "bundled-icons",
        "--target", "wasm32-unknown-unknown")
    shutil.rmtree(DIST, ignore_errors=True)
    DIST.mkdir(parents=True)
    run("wasm-bindgen", "--target", "web", "--no-typescript", "--out-name", "web_landing",
        "--out-dir", DIST / "pkg", WASM)
    shutil.copy(HERE / "index.html", DIST / "index.html")


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                      ".wasm": "application/wasm", ".js": "text/javascript"}

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")  # always the latest build
        super().end_headers()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--port", type=int, default=8080)
    args = parser.parse_args()
    if not args.no_build:
        build()
    if not (DIST / "index.html").exists():
        sys.exit("nothing to serve: run without --no-build first")
    handler = functools.partial(Handler, directory=str(DIST))
    with http.server.ThreadingHTTPServer(("127.0.0.1", args.port), handler) as server:
        print(f"http://localhost:{args.port}/  (Ctrl+C to stop)", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
