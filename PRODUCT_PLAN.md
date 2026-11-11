# Keyless v1 release plan

Keyless makes two physical computers feel like one multi-monitor desktop. The
primary PC owns the keyboard and mouse; a paired AI PC is an adjacent virtual
display.

## Required v1 experience

| Area | Release behavior | Status |
| --- | --- | --- |
| Input | Pointer, clicks, scroll, and keyboard move together | In progress |
| Layout | Four configurable edges; aligned screen geometry | In progress |
| Escape | Edge return and a configurable emergency hotkey | In progress |
| Connection | LAN discovery, explicit pairing, encryption, reconnect | Planned |
| Clipboard | Opt-in text and image synchronization | Planned |
| Files | Explicit file send, receive folder, progress, checksum | In progress |
| UX | Tray status, pairing/setup, display arrangement | Planned |
| Reliability | Start on login, sleep/wake recovery, event log | Planned |
| Safety | Trusted-device list, network restriction, reset/unpair | Planned |

## Performance budget

- No webview, Electron runtime, or background database.
- Native single-process agent per computer.
- Compact fixed-size input messages and no allocation in the hot path.
- Target under 1% CPU while idle and imperceptible pointer handoff on wired LAN.

## Test gate before public release

1. Keyboard, click, drag, wheel, and modifier tests on both PCs.
2. Every edge and mismatched resolution/DPI layout.
3. Disconnect/reconnect while a key or mouse button is held.
4. File integrity, duplicate filenames, interrupted transfer, and large files.
5. Firewall, unpaired-peer, and unsafe-network rejection.
6. Startup, sleep/wake, fullscreen, and accessibility checks.

## Explicit non-goals for the first test build

macOS/Linux support, cloud relay, remote-over-internet control, and unattended
file reception are deferred. They would either broaden the security boundary or
slow the Windows-first experience down.
