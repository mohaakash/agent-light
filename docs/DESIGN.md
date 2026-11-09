# Design and UX Specification

## 1. Design direction

AgentLight should feel like a small, polished developer utility: minimal, calm, legible and unobtrusive.

The virtual traffic light is the visual identity of the product. The dashboard is a supporting control surface.

## 2. Virtual traffic-light window

### Default appearance

- Vertical three-light layout: red, yellow, green.
- Dark neutral housing or a minimal frameless presentation.
- Only the active state glows strongly; inactive lights remain visibly dim.
- Transparent window background where supported.
- Compact footprint.
- No unnecessary text in the default compact mode.

### Interaction

- Drag to reposition.
- Always-on-top toggle.
- Show/hide.
- Click or context menu to access dashboard/settings.
- Optional lock-position setting.
- Remember position and size.
- Avoid accidental clicks through the widget unless click-through is an explicit user setting.

### Customization (P2)

- Classic, minimal and compact themes.
- Scale/size.
- Opacity.
- Light glow intensity.
- Optional agent label.
- Optional aggregate status label.

## 3. Main dashboard

Suggested navigation:
- Overview
- Sessions
- Hardware
- Integrations
- Appearance
- Settings
- Diagnostics

### Overview

Show:
- Current aggregate state.
- Active session count.
- Per-session agent, project label and state.
- Virtual output enabled/disabled.
- Physical device connection state.

### Hardware

Show:
- Detected serial ports.
- Selected device.
- Connection state.
- Firmware/device identity when available.
- Test red, yellow, green and all-off commands.
- Clear disconnect and reconnect actions.

### Integrations

Show:
- Supported agent adapters.
- Installed/not installed/unknown state.
- Setup instructions.
- Enable/disable controls.
- Never silently overwrite user configuration.

## 4. Color and accessibility

Default colors:
- Red: waiting for human attention or error.
- Yellow: working.
- Green: completed.
- Off: idle.

Color must not be the only way to communicate state in the dashboard. Include text labels and accessible names. Provide reduced-glow and color customization options.

## 5. Window behavior

- Main window and floating widget are separate windows.
- The floating window must remain usable when the main window is hidden.
- Closing the main window should follow a user-configurable policy.
- Restore widget position safely if the saved monitor is no longer connected.
- Clamp restored coordinates to a visible display area.
- Test multi-monitor and mixed-DPI behavior on each OS.

## 6. Visual states

The widget should distinguish:
- WAITING
- WORKING
- COMPLETED
- IDLE
- ERROR

WAITING and ERROR both use red in the simple traffic-light representation, but must be distinguishable in expanded UI or accessible labels.
