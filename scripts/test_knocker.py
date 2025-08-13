#!/usr/bin/env python3
import os
import sys
import time
import socket
import shutil
import random
import string
import threading
import subprocess
from dataclasses import dataclass
from typing import List, Optional, Tuple

# ---------------------------------------------------------------------------
# Configuration and helpers
# ---------------------------------------------------------------------------

@dataclass
class RunResult:
    code: int
    out: str
    err: str
    duration_s: float


def is_windows() -> bool:
    return os.name == "nt"


def project_root() -> str:
    # go up one level to project root
    return os.path.abspath(os.path.join(os.path.dirname(__file__), "..")) 


def default_bin_path(debug: bool = True) -> str:
    name = "async_port_knocker" + (".exe" if is_windows() else "")
    target = "debug" if debug else "release"
    return os.path.join(project_root(), "target", target, name)


def build_binary() -> str:
    # Change project build here
    cmd = ["cargo", "build", "-q"]
    print("Building Rust binary with:", " ".join(cmd))
    cp = subprocess.run(
        cmd,
        cwd=project_root(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if cp.returncode != 0:
        print("cargo build failed:")
        print(cp.stdout)
        print(cp.stderr, file=sys.stderr)
        sys.exit(1)
    print("Cargo build successful")

    bin_path = default_bin_path(debug=True)
    if not os.path.exists(bin_path):
        print(f"Built binary not found at {bin_path}")
        sys.exit(1)
    return bin_path


def find_or_build_binary() -> str:
    env_bin = os.environ.get("KNOCKER_BIN")
    if env_bin:
        if os.path.exists(env_bin) and os.access(env_bin, os.X_OK):
            print(f"Using binary from KNOCKER_BIN: {env_bin}")
            return env_bin
        print(f"KNOCKER_BIN set but not executable: {env_bin}")
        sys.exit(1)

    # Try existing debug build first
    bin_path = default_bin_path(debug=True)
    if os.path.exists(bin_path) and os.access(bin_path, os.X_OK):
        print(f"Using existing binary: {bin_path}")
        return bin_path

    # Build if missing
    if shutil.which("cargo") is None:
        print("cargo not found in PATH and no binary specified via KNOCKER_BIN.")
        sys.exit(1)
    return build_binary()


def run_knocker(
    bin_path: str,
    host: str,
    protocol: str,
    sequence: List[int],
    timeout_ms: int = 500,
    delay_ms: int = 0,
    concurrency: int = 1,
    retries: int = 1,
    backoff_ms: int = 100,
    payload_hex: Optional[str] = None,
    extra_args: Optional[List[str]] = None,
    run_timeout_s: float = 30.0,
) -> RunResult:
    args = [
        bin_path,
        "-H",
        host,
        "--protocol",
        protocol,
        "--sequence",
        ",".join(str(p) for p in sequence),
        "--timeout",
        str(timeout_ms),
        "--delay",
        str(delay_ms),
        "--concurrency",
        str(concurrency),
        "-r",
        str(retries),
        "-b",
        str(backoff_ms),
    ]
    if payload_hex is not None:
        args += ["--payload", payload_hex]
    if extra_args:
        args += extra_args

    start = time.monotonic()
    try:
        cp = subprocess.run(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=run_timeout_s,
            cwd=project_root(),
        )
        end = time.monotonic()
        return RunResult(
            code=cp.returncode, out=cp.stdout, err=cp.stderr, duration_s=end - start
        )
    except subprocess.TimeoutExpired as e:
        end = time.monotonic()
        return RunResult(
            code=124,
            out=e.stdout or "",
            err=e.stderr or "Process timeout",
            duration_s=end - start,
        )


# ---------------------------------------------------------------------------
# Local helper servers
# ---------------------------------------------------------------------------

class TcpServer:
    def __init__(self, host: str = "127.0.0.1"):
        self.host = host
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.bind((host, 0))
        self.port = self.sock.getsockname()[1]
        self.sock.listen(5)
        self.sock.settimeout(0.2)
        self.stop_ev = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)

    def _run(self):
        while not self.stop_ev.is_set():
            try:
                conn, _addr = self.sock.accept()
                try:
                    conn.settimeout(0.1)
                    # Read/ignore a bit to keep the connection simple
                    try:
                        conn.recv(16)
                    except Exception:
                        pass
                finally:
                    conn.close()
            except socket.timeout:
                continue
            except OSError:
                break

    def start(self):
        self.thread.start()

    def stop(self):
        self.stop_ev.set()
        try:
            # Nudge accept
            with socket.create_connection((self.host, self.port), timeout=0.2):
                pass
        except Exception:
            pass
        try:
            self.sock.close()
        except Exception:
            pass
        self.thread.join(timeout=1.0)


class UdpEchoServer:
    def __init__(self, host: str = "127.0.0.1", reply_bytes: bytes = b"pong"):
        self.host = host
        self.reply = reply_bytes
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.sock.bind((host, 0))
        self.port = self.sock.getsockname()[1]
        self.sock.settimeout(0.2)
        self.stop_ev = threading.Event()
        self.thread = threading.Thread(target=self._run, daemon=True)

    def _run(self):
        while not self.stop_ev.is_set():
            try:
                data, addr = self.sock.recvfrom(2048)
                try:
                    # Echo back or fixed reply
                    rb = self.reply if self.reply is not None else data
                    self.sock.sendto(rb, addr)
                except Exception:
                    pass
            except socket.timeout:
                continue
            except OSError:
                break

    def start(self):
        self.thread.start()

    def stop(self):
        self.stop_ev.set()
        try:
            # Nudge recv
            self.sock.sendto(b"", (self.host, self.port))
        except Exception:
            pass
        try:
            self.sock.close()
        except Exception:
            pass
        self.thread.join(timeout=1.0)


# ---------------------------------------------------------------------------
# DNS payload builder (for UDP test against 8.8.8.8:53)
# ---------------------------------------------------------------------------

def build_dns_query_hex(qname: str = "example.com", qtype: int = 1) -> str:
    # Minimal DNS query with RD=1
    def pack_name(name: str) -> bytes:
        parts = name.strip(".").split(".")
        out = bytearray()
        for p in parts:
            b = p.encode("ascii")
            out.append(len(b))
            out.extend(b)
        out.append(0)
        return bytes(out)

    qid = random.randint(0, 0xFFFF)
    header = bytearray()
    header += qid.to_bytes(2, "big")        # ID
    header += (0x0100).to_bytes(2, "big")   # Flags: RD=1
    header += (1).to_bytes(2, "big")        # QDCOUNT
    header += (0).to_bytes(2, "big")        # ANCOUNT
    header += (0).to_bytes(2, "big")        # NSCOUNT
    header += (0).to_bytes(2, "big")        # ARCOUNT

    question = bytearray()
    question += pack_name(qname)
    question += qtype.to_bytes(2, "big")    # QTYPE
    question += (1).to_bytes(2, "big")      # QCLASS = IN

    pkt = bytes(header + question)
    return pkt.hex()


# ---------------------------------------------------------------------------
# Test cases
# ---------------------------------------------------------------------------

@dataclass
class TestCase:
    name: str
    fn: callable


def expect(cond: bool, msg: str) -> Tuple[bool, str]:
    return (True, msg) if cond else (False, msg)


def test_tcp_success_local(bin_path: str) -> Tuple[bool, str]:
    srv = TcpServer()
    srv.start()
    try:
        res = run_knocker(
            bin_path,
            host="127.0.0.1",
            protocol="tcp",
            sequence=[srv.port],
            timeout_ms=800,
            retries=1,
        )
        ok = res.code == 0 and f"TCP 127.0.0.1:{srv.port} OK" in res.out
        return expect(ok, f"stdout: {res.out.strip()} stderr: {res.err.strip()}")
    finally:
        srv.stop()


def test_tcp_err_refused(bin_path: str) -> Tuple[bool, str]:
    # Port 1 should be closed on localhost, causing immediate refusal
    port = 1
    res = run_knocker(
        bin_path,
        host="127.0.0.1",
        protocol="tcp",
        sequence=[port],
        timeout_ms=500,
        retries=1,
    )
    ok = res.code == 0 and "OK" not in res.out
    return expect(ok, f"stdout: {res.out.strip()} stderr: {res.err.strip()}")


def test_udp_success_local_echo(bin_path: str) -> Tuple[bool, str]:
    srv = UdpEchoServer(reply_bytes=b"hello")
    srv.start()
    try:
        res = run_knocker(
            bin_path,
            host="127.0.0.1",
            protocol="udp",
            sequence=[srv.port],
            timeout_ms=700,
            retries=1,
        )
        ok = (
            res.code == 0
            and f"UDP 127.0.0.1:{srv.port} received " in res.out
        )
        return expect(ok, f"stdout: {res.out.strip()} stderr: {res.err.strip()}")
    finally:
        srv.stop()


def test_public_tcp_google_443(bin_path: str) -> Tuple[bool, str]:
    if os.environ.get("SKIP_PUBLIC") == "1":
        return expect(True, "Skipped (SKIP_PUBLIC=1)")
    res = run_knocker(
        bin_path,
        host="www.google.com",
        protocol="tcp",
        sequence=[443],
        timeout_ms=1500,
        retries=1,
    )
    ok = res.code == 0 and "OK" in res.out
    return expect(ok, f"stdout: {res.out.strip()} stderr: {res.err.strip()}")


def test_public_udp_dns_query(bin_path: str) -> Tuple[bool, str]:
    if os.environ.get("SKIP_PUBLIC") == "1":
        return expect(True, "Skipped (SKIP_PUBLIC=1)")
    payload_hex = build_dns_query_hex("example.com", qtype=1)
    res = run_knocker(
        bin_path,
        host="8.8.8.8",
        protocol="udp",
        sequence=[53],
        timeout_ms=1500,
        retries=1,
        payload_hex=payload_hex,
    )
    # Expect a reply from DNS resolver
    ok = res.code == 0 and "UDP 8.8.8.8:53 received " in res.out
    return expect(ok, f"stdout: {res.out.strip()} stderr: {res.err.strip()}")


def test_invalid_payload_hex(bin_path: str) -> Tuple[bool, str]:
    res = run_knocker(
        bin_path,
        host="127.0.0.1",
        protocol="udp",
        sequence=[9],
        timeout_ms=300,
        retries=1,
        payload_hex="xyz",  # invalid hex
    )
    ok = res.code != 0
    return expect(ok, f"code={res.code} stdout: {res.out.strip()} "
                      f"stderr: {res.err.strip()}")


def test_dns_resolution_error(bin_path: str) -> Tuple[bool, str]:
    # .invalid TLD is reserved and guaranteed to fail DNS
    res = run_knocker(
        bin_path,
        host="nonexistent.invalid",
        protocol="tcp",
        sequence=[80],
        timeout_ms=500,
        retries=1,
    )
    ok = res.code != 0 and "Error:" in res.err
    return expect(ok, f"code={res.code} stdout: {res.out.strip()} "
                      f"stderr: {res.err.strip()}")


def test_concurrency_udp_timeout(bin_path: str) -> Tuple[bool, str]:
    # Use a public host/port that will not reply (UDP discard-like)
    # This makes total duration ~timeout per parallel group.
    host = "8.8.8.8"
    if os.environ.get("SKIP_PUBLIC") == "1":
        # Fallback to localhost closed UDP port; may produce immediate errors.
        host = "127.0.0.1"

    # Two knocks that should both wait until timeout if no reply
    seq = [9, 19]
    to_ms = 700

    res_seq = run_knocker(
        bin_path,
        host=host,
        protocol="udp",
        sequence=seq,
        timeout_ms=to_ms,
        retries=1,
        concurrency=1,
    )
    dur_seq = res_seq.duration_s

    res_par = run_knocker(
        bin_path,
        host=host,
        protocol="udp",
        sequence=seq,
        timeout_ms=to_ms,
        retries=1,
        concurrency=2,
    )
    dur_par = res_par.duration_s

    # Expect parallel run to be significantly faster
    ok = dur_par < dur_seq * 0.75
    msg = (f"sequential={dur_seq:.3f}s parallel={dur_par:.3f}s "
           f"stdout(seq)={res_seq.out.strip()} "
           f"stdout(par)={res_par.out.strip()}")
    return expect(ok, msg)


def test_retries_behavior_udp(bin_path: str) -> Tuple[bool, str]:
    # Use a local UDP server that does NOT reply to simulate timeouts,
    # so we can observe multiple attempts in total runtime.
    class SilentUdpServer(UdpEchoServer):
        def _run(self):
            while not self.stop_ev.is_set():
                try:
                    # Read and DO NOT reply
                    self.sock.recvfrom(2048)
                except socket.timeout:
                    continue
                except OSError:
                    break

    srv = SilentUdpServer()
    srv.start()
    try:
        retries = 3
        backoff_ms = 150
        to_ms = 300

        res = run_knocker(
            bin_path,
            host="127.0.0.1",
            protocol="udp",
            sequence=[srv.port],
            timeout_ms=to_ms,
            retries=retries,
            backoff_ms=backoff_ms,
        )
        # Minimal expected duration: retries * timeout + (retries-1) * backoff
        min_expected = (retries * to_ms + (retries - 1) * backoff_ms) / 1000.0
        ok = res.duration_s >= (min_expected * 0.9)  # allow some slack
        msg = (f"duration={res.duration_s:.3f}s min_expected={min_expected:.3f}s "
               f"stdout={res.out.strip()} stderr={res.err.strip()}")
        return expect(ok, msg)
    finally:
        srv.stop()


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------

def main():
    bin_path = find_or_build_binary()
    tests: List[TestCase] = [
        TestCase("TCP local success", lambda: test_tcp_success_local(bin_path)),
        TestCase("TCP local refused", lambda: test_tcp_err_refused(bin_path)),
        TestCase("UDP local echo success",
                 lambda: test_udp_success_local_echo(bin_path)),
        TestCase("Public TCP google:443",
                 lambda: test_public_tcp_google_443(bin_path)),
        TestCase("Public UDP DNS query 8.8.8.8:53",
                 lambda: test_public_udp_dns_query(bin_path)),
        TestCase("Invalid payload hex", lambda: test_invalid_payload_hex(bin_path)),
        TestCase("DNS resolution error", lambda: test_dns_resolution_error(bin_path)),
        TestCase("Concurrency UDP timing",
                 lambda: test_concurrency_udp_timeout(bin_path)),
        TestCase("UDP retries/backoff timing",
                 lambda: test_retries_behavior_udp(bin_path)),
    ]

    passed = 0
    failed = 0
    print("\n=== async_port_knocker functional tests ===\n")
    for t in tests:
        print(f"- {t.name} ...", end=" ", flush=True)
        ok, msg = t.fn()
        if ok:
            print("PASS")
            print(f"  {msg}")
            passed += 1
        else:
            print("FAIL")
            print(f"  {msg}")
            failed += 1
        print()

    total = passed + failed
    print(f"Summary: {passed}/{total} passed, {failed} failed.")
    if failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()