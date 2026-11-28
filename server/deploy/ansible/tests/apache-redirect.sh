#!/usr/bin/env bash
# Run with bash on Ubuntu with apache2, curl, and Python 3 installed.
set -eu

python3 - "$(dirname "$0")/../roles/cascade_sync/templates/apache-vhost.conf.j2" <<'PY'
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

host = "sync.example.test"
source = Path(sys.argv[1]).read_text()
with tempfile.TemporaryDirectory(prefix="cascade-apache-redirect-") as tmp:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    conf = Path(tmp) / "httpd.conf"
    modules = ["mpm_event", "authz_core", "proxy", "proxy_http", "rewrite"]
    conf.write_text(
        f'ServerRoot "{tmp}"\nListen 127.0.0.1:{port}\nServerName localhost\n'
        f'DocumentRoot "{tmp}"\nPidFile "{tmp}/httpd.pid"\nErrorLog "{tmp}/error.log"\n'
        + "".join(f"LoadModule {m}_module /usr/lib/apache2/modules/mod_{m}.so\n" for m in modules)
        + source.replace("{{ ansible_managed }}", "Test fixture")
        .replace("{{ cascade_sync_host }}", host)
        .replace("{{ cascade_sync_port }}", "3470")
        .replace("*:80>", f"*:{port}>")
        .replace("/var/log/httpd/", tmp + "/")
    )
    server = subprocess.Popen(["/usr/sbin/apache2", "-X", "-f", str(conf)])
    try:
        for _ in range(50):
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=1):
                    break
            except ConnectionRefusedError:
                if server.poll() is not None:
                    raise SystemExit(f"Apache exited with status {server.returncode}")
                time.sleep(0.05)
        else:
            raise SystemExit("Apache did not start")

        for method, target, path in (
            ("OPTIONS", "*", "/"),
            ("POST", "/auth/request?q=a%2Bb", "/auth/request?q=a%2Bb"),
            ("GET", "/encoded%23%3F%20", "/encoded%23%3F%20"),
            ("GET", "/metrics", "/metrics"),
        ):
            actual = subprocess.check_output(
                ["curl", "--noproxy", "*", "--max-time", "2", "-sS", "-o", "/dev/null",
                 "-w", "%{http_code} %{redirect_url}", "-X", method, "--request-target", target,
                 "-H", f"Host: {host}", f"http://127.0.0.1:{port}/"],
                text=True,
            )
            expected = f"301 https://{host}{path}"
            if actual != expected:
                raise SystemExit(f"Expected {expected} for {method} {target}, got {actual.strip()}")
            print(f"{method} {target} -> {actual}")
    finally:
        server.terminate()
        server.wait(timeout=5)
PY
