# Keyless

## Two-PC setup (0.2 preview)

Download `downloads/keyless-windows-x64.exe` and double-click it. The native
Windows setup window discovers nearby IPv4 devices, displays their hostname,
IP and MAC, and lets you edit nicknames. Select exactly two computers and use
**Use selected as host** for the computer with your keyboard and mouse.

**Install selected** installs per-user copies and desktop shortcuts. Remote
installation requires Windows remoting already configured on the destination,
an administrator account, and that account's desktop session for the shortcut.
It does not turn on remoting, bypass Windows security, or launch input injection
inside the remote service session. Start each desktop shortcut locally.

If remoting is unavailable, **Create portable setup folder** produces one folder
to copy to both PCs with ready-to-run host and client launchers. A PC that has no
Keyless receiver or Windows management channel needs this initial copy once.

Discovery probes the local /24 on each IPv4 adapter and reads the neighbor
cache. Sleeping, filtered, isolated or wider-subnet devices may be absent; it
cannot guarantee an inventory of every PC. Hardware/OS details require an
authenticated management session; setup reports the OS after remote install.

This is a setup preview. The input engine still has known limitations: polling
does not suppress local input, wheel capture is not implemented, and seamless
edge return is pending. Sessions and file transfers currently lack authentication
and encryption. Use only on a trusted private LAN. The roadmap below describes
the intended finished product, not features already shipped.

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

