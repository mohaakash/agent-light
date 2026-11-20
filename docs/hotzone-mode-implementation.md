# Hotzone Mode Implementation Plan

## Overview
Implement a GNOME corner-style hotzone feature that allows traffic lights to "peek" from designated hotzone areas when the mouse enters them, with smooth animations. This provides an alternative to the always-on-top mode.

## Current State
- Application has "always on top" mode as the primary visibility option
- Each agent (codex, claude, cursor, etc.) has its own traffic light window
- Windows are draggable and customizable (size, theme, opacity)

## Requirements
1. **Hotzone Mode**: GNOME-corner style area where mouse hover reveals traffic light
2. **Animation**: Traffic light "peeks" from hotzone with smooth animation
3. **Multiple Agents**: Each agent can have its own hotzone area
4. **Multiple Displays**: Hotzones support across multiple monitors
5. **Graceful Fallback**: When hotzone mode is disabled, revert to existing behavior

## Architecture Changes

### 1. Data Models

#### HotZone Structure
```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HotZone {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor_id: Option<usize>,
}

impl HotZone {
    /// Create a default hotzone in the bottom-left corner (GNOME-style)
    pub fn default_bottom_left() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
            monitor_id: None,
        }
    }

    /// Check if a point (x, y) is within this hotzone
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < (self.x + self.width as i32) &&
        y >= self.y && y < (self.y + self.height as i32)
    }
}
```

#### AppSettings Updates
```rust
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct AppSettings {
    pub traffic_light_always_on_top: bool,
    pub traffic_light_position: Option<WidgetPosition>,
    #[serde(default)]
    pub agent_widget_positions: HashMap<String, WidgetPosition>,
    pub hotzone_mode: bool,
    #[serde(default)]
    pub agent_hotzones: HashMap<String, HotZone>,
    pub default_hotzone: HotZone,
    pub widget_size: WidgetSize,
    pub widget_theme: WidgetTheme,
    pub widget_opacity: u8,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            traffic_light_always_on_top: true,
            traffic_light_position: None,
            agent_widget_positions: HashMap::new(),
            hotzone_mode: false,
            agent_hotzones: HashMap::new(),
            default_hotzone: HotZone::default_bottom_left(),
            widget_size: WidgetSize::Standard,
            widget_theme: WidgetTheme::Dark,
            widget_opacity: 100,
        }
    }
}
```

### 2. Tauri Commands

#### Hotzone Mode Commands
- `get_hotzone_mode()` - Get current hotzone mode state
- `set_hotzone_mode(enabled: bool)` - Toggle hotzone mode
- `subscribeToHotzoneMode(handler)` - Event listener for mode changes

#### Per-Agent Hotzone Commands
- `get_agent_hotzone(agent: String)` - Get hotzone for specific agent
- `set_agent_hotzone(agent: String, hotzone: HotZone)` - Configure hotzone

#### Collision Detection
- `get_active_hotzone_at(x: i32, y: i32)` - Find which hotzone contains point
- `get_monitor_at(x: i32, y: i32)` - Detect which monitor contains point

### 3. Component Changes

#### TrafficLightWindow.tsx Updates
- Mouse position tracking via pointer events
- Collision detection with hotzones
- Animation state: `peeking`, `hiding`, `idle`
- CSS transforms for "peeking" animation
- Conditional rendering based on hotzone state

#### Hotzone Indicator (new component)
- Visual indicator showing hotzone location
- Active hotzone highlight
- Current mouse cursor indicator

### 4. CSS Animations

```css
@keyframes peek {
  0% { transform: scaleY(0) translateY(100%); opacity: 0; }
  50% { transform: scaleY(1.1) translateY(95%); opacity: 1; }
  100% { transform: scaleY(1) translateY(0); opacity: 1; }
}

@keyframes hide {
  from { transform: scaleY(1) translateY(0); opacity: 1; }
  to { transform: scaleY(0) translateY(100%); opacity: 0; }
}

.traffic-light-housing.peeking {
  animation: peek 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

### 5. UI Management

#### Hotzone Editor (in App.tsx)
- Toggle for "Hotzone mode"
- Visual hotzone editor (click to place, drag to resize)
- List of agents with their hotzone positions
- Hotzone preview with current mouse position indicator
- Hotzone size adjustment

## User Experience Flow

### 1. Enable Hotzone Mode
- User toggles "Hotzone mode" in settings
- Main traffic light window shows default hotzone indicator
- All agent windows hide (hotzone mode vs always-on-top)

### 2. Configure Hotzones
- User clicks on a hotzone to open editor
- User drags to resize hotzone
- User assigns hotzone to a specific agent
- Changes persist to settings

### 3. Use Hotzone Mode
- User moves mouse to hotzone
- Traffic light "peeks" out from hotzone with animation
- User can click/interact with lights
- After mouse leaves hotzone (2-3 second debounce), light hides
- Or: light stays visible until explicitly hidden

### 4. Multi-Monitor Support
- Hotzones are per-monitor
- User can create separate hotzones on each display
- Traffic light appears on the monitor containing the mouse position

## File Structure

```
src/
├── lib/
│   └── hotzone.ts - Hotzone utilities & API
├── windows/
│   └── traffic-light/
│       ├── TrafficLightWindow.tsx (UPDATED)
│       ├── hotzone-indicator.tsx (NEW)
│       └── hotzone.css (NEW)
└── app/
    └── App.tsx (UPDATED) - Add hotzone management UI

src-tauri/
├── src/
│   ├── core/
│   │   └── mod.rs (UPDATED) - Add HotZone struct & update AppSettings
│   └── commands/
│       └── windows.rs (UPDATED) - Add hotzone commands
└── tauri.conf.json (UPDATED)
```

## Implementation Phases

### Phase 1: Backend Foundation ✅
- [x] Add HotZone data model
- [x] Update AppSettings with hotzone mode
- [x] Add Tauri commands for hotzone management
- [x] Implement hotzone collision detection
- [x] Create hotzone TypeScript API
- [x] Update settings tests for new fields
- [x] HotZone contains() tests

### Phase 2: Main Traffic Light ✅
- [x] Add mouse tracking to main traffic light window
- [x] Implement peeking animation (CSS keyframes)
- [x] Add hotzone visibility toggle
- [x] Create HotZoneIndicator component
- [x] Add hotzone.css with peek/hide animations
- [x] 2-second debounce before hiding

### Phase 3: Agent Windows ✅
- [x] Add per-agent hotzone support
- [x] Implement agent-specific peeking logic
- [x] Per-agent hotzone configuration via commands

### Phase 4: UI Management ✅
- [x] Create hotzone editor UI in App.tsx
- [x] Add visual hotzone indicators
- [x] Hotzone mode toggle button
- [x] Per-agent hotzone configuration panel
- [x] Hotzone position/size controls
- [x] Hotzone editor CSS styles

### Phase 5: Polish ✅
- [x] Multiple monitor support via `get_available_monitors`, `get_monitor_at`
- [x] Hotzone preview overlay in editor UI
- [x] Smooth transitions on mode switch
- [x] Hotzone presets (6 corner/edge positions: top/bottom-left/right, center-left/right)
- [x] Configurable peek delay (500ms-5000ms slider)
- [x] Monitor info display in UI
- [x] Preset selector with size control
- [x] 31/31 tests passing (including HotZonePreset tests)

## Backwards Compatibility

- Default settings: `hotzone_mode: false`
- Old `traffic_light_always_on_top` setting preserved
- Existing window positions still work when hotzone disabled
- Graceful upgrade path for existing users

## Key Design Decisions

1. **Hotzone Size**: Default to 100-150px from screen edge (GNOME style)
2. **Peek Behavior**:
   - Option A: Immediate hide on mouse exit
   - Option B: Stay visible for 2-3 seconds after exit (more usable)
   - **Recommended**: Configurable with default 2s delay
3. **Clicking Through**: Hotzone should pass through clicks to underlying windows
4. **Fallback**: When hotzone mode disabled, existing "always on top" behavior restored
5. **Synchronization**: All agent windows listen to hotzone mode changes

## Future Enhancements

1. **Multi-Monitor Support**: Proper monitor detection using Tauri's `window.available_monitors()`
2. **Hotzone Templates**: Presets for different corner positions (GNOME, macOS, Windows)
3. **Hotzone Groups**: Share hotzones across multiple agents
4. **Smart Positioning**: Auto-position hotzones based on available screen space
5. **Custom Animations**: User-defined animation curves
6. **Sound Effects**: Audio feedback on hotzone hover