# GSP 370 control protocol

This is the working protocol used by `gsp370-tray` for the Sennheiser/EPOS
GSP 370 wireless headset (USB `1395:009a`).  It is an unofficial,
interoperability-oriented reconstruction.  It is not a firmware update
protocol and does not document any command that was not observed in the
official software's traffic or in an independent implementation.

## Device layout

The dongle enumerates at full speed with six interfaces:

| Interface | Class | Driver | Purpose |
|---:|---|---|---|
| 0 | USB Audio control | `snd-usb-audio` | Audio controls (playback/capture volume) |
| 1 | USB Audio streaming | `snd-usb-audio` | Playback |
| 2 | USB Audio streaming | `snd-usb-audio` | Capture |
| 3 | HID, boot keyboard subclass | `usbhid` | Consumer-control media keys only |
| 4 | CDC ACM control | `cdc_acm` | **Settings serial port** (`/dev/ttyACM0`) |
| 5 | CDC ACM data | `cdc_acm` | Data pair for interface 4 |

The HID report descriptor on interface 3 is 37 bytes and declares usage page
`0x0C` (Consumer), usage `0x01` (Consumer Control), with bits for play/pause,
next, previous, volume up/down and mute.  All settings traffic goes over the
CDC-ACM port, not HID.

`/sys/class/tty/ttyACM*/device/..` is the USB device; `idVendor`/`idProduct`
identify it as `1395`/`009a`.  That is how the app auto-detects the port.

## Serial settings

- 921600 baud, 8 data bits, no parity, 1 stop bit, no flow control
- raw mode (`cfmakeraw`), `CLOCAL | CREAD`
- the dongle needs a short settle after opening (the app uses 150 ms)
- commands are paced: the app sleeps 100 ms before each command (including
  the second half of a sidetone set).  Both independent implementations use
  the same 100 ms pacing; without it, back-to-back commands are unreliable.
- reply timeout: 1500 ms in this app (other implementations use 1–2 s)

`ModemManager` must not probe the port.  The supplied udev rule sets
`ID_MM_DEVICE_IGNORE`/`ID_MM_CANDIDATE=0`.

## Framing

All multi-byte examples are hex.  Frames look like Bluetooth HCI vendor
commands, but this is a proprietary serial protocol with the same shape.

```
command:  01 <opcode> fc <len> <params...>
reply:    04 0e <len> 01 <opcode> fc <status> <values...>
```

`<len>` is the number of bytes after the length byte.  For a reply,
`len = 4 + number_of_value_bytes` (the four bytes `01 op fc status`).

Battery query and a healthy reply at 87 %:

```
TX  01 9c fc 00
RX  04 0e 05 01 9c fc 00 57
                  |  |  |
                  |  |  +-- 0x57 = 87 %
                  |  +----- status 00 = success
                  +-------- opcode 9c
```

The dongle also emits asynchronous events.  The two seen in captures are
`04 e9 00` and `04 e8 00`; their fields are not decoded.  The parser scans the
input stream for the matching reply header and skips everything else.  A
partial matching frame is never parsed as complete.

Status bytes:

| Status | Meaning |
|---:|---|
| `0x00` | success |
| `0x0C` | headset is switched off or out of range |
| other | command-specific failure; treated as an error |

## Commands

| Opcode | Params | Reply values | Meaning | Confidence |
|---:|---|---|---|---|
| `0x9C` | none | 1 byte: `0..100` | Battery percentage | observed |
| `0xA0` | none | none | Prepare/set sidetone to a non-zero level | observed sequence |
| `0xA1` | none | none | Prepare/set sidetone off | observed sequence |
| `0xA2` | 1 byte: level | none | Apply sidetone level | observed |

A sidetone update is two commands:

```
TX  01 a0 fc 00                 # non-zero level
RX  04 0e 04 01 a0 fc 00
TX  01 a2 fc 01 0a              # level 0x0a
RX  04 0e 04 01 a2 fc 00
```

For off, the first command is `A1`, then `A2 00`.  The exact distinction
between `A0`/`A1` and `A2` (enable/mute vs. prepare/apply) is inferred from
the order and from the level parameter; it is not documented by EPOS.

### Sidetone scale

The device accepts `0` (off) or levels `6..16`.  Values `1..5` are skipped.
The official suite's slider has 128 user steps mapped onto that range.  This
app exposes 0–100 % and maps:

```
level = 0                                  if percent == 0
level = 6 + (percent - 1) * 10 / 99        otherwise (integer arithmetic)
```

So 10 % -> 6, 50 % -> 10, 100 % -> 16.  The headset does not report the
current sidetone level; `0xA3` returned the same value after audible changes
in prior captures.  The tray therefore shows the last value *set by this app*,
not a read-back.

Sidetone is only audible while the microphone stream is open.  This is a
firmware behaviour, not something the serial command can change.

## What the official Windows suite does for this model

The Gaming Suite (1.12.2.1185) maps the GSP 370 to the `NXP` firmware adapter
(`DeviceInfoConfig.txt`; `GSP_370`/`EPOS_GSP_370`, decimal VID/PID
`5013`/`154` and `5013`/`171`).  The native service contains
`NxpProtocol::get_battery_level`, `NxpProtocol::get_sidetone_level`,
`CNXPAdapter::GetCOMPort`, `CNXPAdapter::getBatteryPercentage` and
`CNXPAdapter::setSideToneValue`; the UI contains GSP 370-specific sidetone
handling (`SidetoneValueGSP370`, `SidetoneKnobGSP370`) and two firmware-update
paths (headset and dongle).

For this model the hardware-facing feature set is therefore:

- battery percentage
- sidetone level, with no read-back
- headset firmware update and dongle firmware update (separate components)

The suite's EQ, surround, reverb, noise gate and microphone processing are
Windows APO/software effects.  They are not stored in the GSP 370's NXP
firmware and are outside the scope of this app.  Firmware flashing is also
deliberately not implemented: a wrong image or timing can brick the dongle or
headset, and the protocol was not validated for that purpose here.

## Evidence

This reconstruction combines:

1. The official EPOS Gaming Suite 1.12.2.1185 MSI
   (SHA-256 `b1634892d5a1c008b2d20d259f36a51ec61751955975031ce67e24cc14e2969e`),
   specifically `DeviceInfoConfig.txt`, `EPOSGamingSuiteService.exe` and
   `EPOSGamingSuite.exe`.
2. USB descriptors and sysfs on the test machine (Linux 7.3.1, CachyOS):
   `1395:009a`, interfaces as listed above, `/dev/ttyACM0` created by
   `cdc_acm`.
3. The public HeadsetControl GSP 370 work
   (`kaffeewolf/HeadsetControl`, branch `add-gsp370-support`, commit
   `e9231bf2677382b1254aa082240e2c6a015ae101`), which contains captured
   command bytes and a tested Linux implementation.
4. The independent `eposctl` GSP 370 implementation
   (`hiyufan/eposctl`, commit `783d8166f49ed9f198f6c5efeef90415b5576ec6`),
   which documents the same framing and commands but marks the model as
   untested.
5. This repository's pseudo-terminal integration test
   (`tests/e2e_pty.py`), which exercises termios setup, framing, event
   skipping, error statuses and the sidetone sequence without hardware.
6. Hardware checks on 2026-11-04 with the unit above: the Rust tray's
   `--probe` returned `status=on battery=84`; `--set-sidetone 30` succeeded
   with status 0; the tray menu then set 30 % and persisted it.  The Qt
   version had previously confirmed battery/sidetone as well, so the dongle
   replies are consistent with the published captures.

The command set is consistent across the independent sources.  The
hardware-verification result for the unit used during development is recorded
in the project README.

## Unknowns / deliberately unprobed

- Meaning of `A0`/`A1` beyond the observed sequence.
- Whether sidetone persists across a headset power cycle.
- Battery reporting granularity and whether a charging flag exists.
- Other opcodes (firmware version, serial number, pairing, DFU).  Sending
  unknown opcodes is avoided, especially anything that could enter DFU.
- The asynchronous `04 e8`/`04 e9` events.
- Firmware update framing and image format.
