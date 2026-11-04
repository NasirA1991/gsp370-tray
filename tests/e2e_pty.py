#!/usr/bin/env python3
"""End-to-end test of the serial framing against a pseudo-terminal dongle.

This does not need the real headset.  It exercises Device::open(), termios
configuration, command writes, reply framing, events-before-reply, and the
CLI's exit codes.  Run it after building:

    python3 tests/e2e_pty.py ./build/gsp370-tray
"""

from __future__ import annotations

import argparse
import os
import pty
import select
import subprocess
import sys
import termios
import time
import tty


class FakeDongle:
    def __init__(self) -> None:
        self.master, self.slave = pty.openpty()
        tty.setraw(self.master)
        tty.setraw(self.slave)
        self.path = os.ttyname(self.slave)
        self.buffer = b""

    def close(self) -> None:
        for fd in (self.master, self.slave):
            try:
                os.close(fd)
            except OSError:
                pass

    def read_exact(self, count: int, timeout: float = 3.0) -> bytes:
        deadline = time.monotonic() + timeout
        while len(self.buffer) < count:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise AssertionError(
                    f"timed out waiting for {count} bytes; have {self.buffer!r}"
                )
            ready, _, _ = select.select([self.master], [], [], remaining)
            if not ready:
                raise AssertionError(
                    f"timed out waiting for {count} bytes; have {self.buffer!r}"
                )
            chunk = os.read(self.master, 256)
            if not chunk:
                raise AssertionError("dongle master closed")
            self.buffer += chunk
        result, self.buffer = self.buffer[:count], self.buffer[count:]
        return result

    def next_command(self, timeout: float = 3.0) -> tuple[int, bytes]:
        header = self.read_exact(4, timeout)
        if header[0] != 0x01 or header[2] != 0xFC:
            raise AssertionError(f"bad command header {header!r}")
        opcode = header[1]
        length = header[3]
        params = self.read_exact(length, timeout) if length else b""
        return opcode, params

    def reply(self, opcode: int, status: int = 0, values: bytes = b"") -> None:
        payload_len = 4 + len(values)  # 01 op fc status values
        frame = bytes((0x04, 0x0E, payload_len, 0x01, opcode, 0xFC, status)) + values
        os.write(self.master, frame)

    def event(self) -> None:
        os.write(self.master, b"\x04\xe9\x00")


def run_cli(binary: str, path: str, *args: str) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env.pop("GSP370_TTY", None)
    return subprocess.run(
        [binary, *args, "--device", path],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=15,
        env=env,
    )


def test_battery(binary: str) -> None:
    dongle = FakeDongle()
    try:
        proc = subprocess.Popen(
            [binary, "--battery", "--device", dongle.path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        dongle.event()
        opcode, params = dongle.next_command()
        assert opcode == 0x9C, hex(opcode)
        assert params == b"", params
        dongle.reply(0x9C, 0, bytes((87,)))
        out, err = proc.communicate(timeout=10)
        assert proc.returncode == 0, (proc.returncode, out, err)
        assert out.strip() == "87", repr(out)
    finally:
        dongle.close()


def test_headset_off(binary: str) -> None:
    dongle = FakeDongle()
    try:
        proc = subprocess.Popen(
            [binary, "--battery", "--device", dongle.path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        opcode, _ = dongle.next_command()
        dongle.reply(opcode, 0x0C)
        out, err = proc.communicate(timeout=10)
        assert proc.returncode == 2, (proc.returncode, out, err)
        assert out.strip() == "off", repr(out)
        assert "switched off" in err, repr(err)
    finally:
        dongle.close()


def test_sidetone(binary: str) -> None:
    dongle = FakeDongle()
    try:
        proc = subprocess.Popen(
            [binary, "--set-sidetone", "50", "--device", dongle.path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        first_op, first_params = dongle.next_command()
        assert first_op == 0xA0, hex(first_op)
        assert first_params == b"", first_params
        dongle.reply(0xA0)

        second_op, second_params = dongle.next_command()
        assert second_op == 0xA2, hex(second_op)
        assert second_params == b"\x0a", second_params  # 50 % -> level 10
        dongle.reply(0xA2)

        out, err = proc.communicate(timeout=10)
        assert proc.returncode == 0, (proc.returncode, out, err)
        assert out.strip() == "sidetone=50%", repr(out)
    finally:
        dongle.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("binary", nargs="?", default="./build/gsp370-tray")
    args = parser.parse_args()

    for test in (test_battery, test_headset_off, test_sidetone):
        test(args.binary)
        print(f"PASS {test.__name__}")
    print("All PTY end-to-end tests passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
