# Keyless test checklist

Run both machines on a trusted private LAN. Start the AI computer first:

```powershell
.\keyless-windows-x64.exe client <PRIMARY-PC-IP>
```

Then start the primary PC:

```powershell
.\keyless-windows-x64.exe host
```

## Input handoff

- Move to the primary display's right edge. Confirm the AI PC receives pointer
  movement, clicks, scrolling, and normal typing.
- Press `Ctrl+Alt+Space`; confirm control returns to the primary PC.
- Test Shift, Ctrl, Alt, punctuation, and a long held key.
- Test left/right/middle buttons, wheel scrolling, click-and-drag, and a
  fullscreen app on the AI PC.

## Connection recovery

- Stop the AI client, then restart it and reconnect the host.
- Temporarily disable the private network on either PC; verify the processes
  fail safely without locking the primary keyboard.

## File transfer

- With the AI client running, send a small text file and a larger file:

  ```powershell
  .\keyless-windows-x64.exe send <AI-PC-IP> "C:\path\to\file"
  ```

- Confirm the result appears in `Keyless Files` next to the AI executable.
- Send the same filename twice; confirm both are retained.
- Interrupt a large transfer; confirm no incomplete final file appears.

Please record the exact command, Windows version, display resolutions, and
what happened for any failure. That makes a useful next build possible.
