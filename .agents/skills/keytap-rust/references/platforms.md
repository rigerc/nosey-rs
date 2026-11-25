# Platform notes and troubleshooting

## macOS

- A global listener can observe keys while another app is focused, subject to OS privacy restrictions.
- Initializing the tap may return `Error::PermissionDenied` when required Input Monitoring / Accessibility permission is missing.
- Check **System Settings > Privacy & Security > Input Monitoring** and, if needed, **Accessibility** for the exact executable/host application; launch again after granting permission if the OS requires it.
- For developer tooling, the actual host terminal, IDE, or packaged app may be the process requiring permission; do not assume a debug binary and a packaged app share privacy grants.

## Linux, including Wayland

- The Linux implementation uses evdev (`/dev/input/event*`), not compositor-specific Wayland shortcut APIs.
- Device readability is a significant permission boundary. A session that can read raw input events may see keystrokes from other apps. Explain that implication; request explicit consent where necessary.
- `PermissionDenied` means keyboards were present but inaccessible; `NoDevices` means no usable keyboard devices were discovered. Containers, sandboxes, remote sessions, and udev rules can affect access.
- Prefer least-privilege udev/device-policy approaches appropriate to the application's deployment. Avoid suggesting `sudo` or permissive device permissions as an automatic fix.
- Account for unplug/replug, multiple keyboards, laptop suspend/resume, and events missed while the app was paused.

## Windows

- The backend uses a system low-level keyboard hook (`WH_KEYBOARD_LL`). App focus is not required for observed input in the interactive session.
- Secure desktop, session boundaries, OS policies, and special privilege boundaries can prevent delivery. Do not claim omniscient key capture.
- Monitor listener health and ensure app lifecycle does not drop the `Tap` prematurely.

## State model and failures

- Do not assume a key that was down before monitoring started will emit a new down event.
- A missed `KeyUp` can leave a derived `HashSet<Key>` stale; plan a fail-safe cancellation or reset on interruption / restart, particularly for privacy-sensitive continuous capture.
- `KeyRepeat` is not another newly pressed key; ignore for one-shot activations.
- Distinguish **background focus** (supported by the OS tap with appropriate permissions) from **background process execution** (the process must remain running), and from **systemwide security boundaries** (not guaranteed).
- For UX, display a transparent active-monitoring indicator and avoid logging, transmitting, or storing arbitrary global input events.

## Source

- https://docs.rs/keytap/0.4.0/keytap/enum.Error.html
- https://github.com/jamiepine/keytap/blob/main/DESIGN.md
