# Keyless

**Two computers. One keyboard and mouse.**

Keyless makes a local AI machine behave like the next monitor in a desktop
layout. Moving the pointer through a configured display edge transfers pointer
and keyboard control to the paired computer; returning through its matching
edge gives control back.

## Product principles

- Tray-first: no permanent window or browser runtime.
- Fast path: input events are compact and avoid allocations.
- Intentional: the pointer only crosses where the displays actually touch.
- Private: paired machines communicate directly on the local network.
- Windows-first: native input capture/injection in the first release.

## Architecture

```text
Windows input hook -> Router -> encrypted LAN transport -> Input injector
                         ^                                  |
                         +----------- acknowledgement ------+
```

The crate currently contains platform-independent routing and protocol code.
The next layers are the Windows input adapter and authenticated LAN transport.

## Run it

On the keyboard/mouse PC run `keyless.exe host`. On the AI PC run
`keyless.exe client <PRIMARY-PC-IP>`. Move through the primary screen's right
edge to hand off and press `Ctrl+Alt+Space` to toggle control back.

### Transfer a file

While the AI PC is running in client mode, send from the primary PC:

```powershell
.\keyless.exe send <AI-PC-IP> "C:\path\to\file.exe"
```

The AI PC receives it in a `Keyless Files` folder beside the executable.

## Development

```powershell
cargo test
cargo run
```

