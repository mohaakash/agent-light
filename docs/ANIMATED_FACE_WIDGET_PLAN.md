# Animated Face Widget — Implementation Plan

**Status:** MVP implemented in the current branch. Packaged-runtime and manual visual QA remain.

## Goal

Offer an animated Taby face as an optional virtual-widget presentation while keeping the existing traffic light unchanged and selected by default. Users can switch between **Traffic light** and **Animated face** in the existing Virtual Lights settings. The face follows the same per-agent state, position, visibility, focus, hotzone, and always-on-top behavior as the current widget.

The initial implementation should use the Taby 1.64 animation pack from `C:/Users/Akash/projects/firmware-taby/assets/amoled-1.64`. It should not require a physical Taby device or firmware.

## Findings from `firmware-taby`

- The Taby 1.64 pack is **280×456**, catalog version **0.4.1**, with **84 GIF animations** described in `assets/amoled-1.64/catalog.json`. The animation files are hash-named; the catalog supplies stable IDs, file paths, durations, loop policies, and hashes.
- The pack is about **12 MB** on disk. A small group of status-relevant animations totals around **1 MB**, so the first release can avoid copying/bundling unrelated clips.
- The sample GIFs report no GIF transparency index and display on black. Treat the art as an opaque AMOLED-style image; do not remove black with chroma-key blending. Preview it in a dark, rounded widget frame.
- The firmware uses state-to-animation mappings and distinguishes `loop` from `play_once`; it transitions to another scene after one-shot animations. Its asset lookup also keeps animation IDs stable even when files use hashed paths. The desktop implementation should follow these ideas, without importing firmware code.
- The source includes a **round 1.32** pack whose render profile describes a transformed/prototype artwork. Start with the rectangular 1.64 pack; consider the round pack only after a separate visual review.
- Per the request, use the Taby assets in this open-source integration and add clear Taby/TRIIIS LABS attribution to AgentLight's README. Keep Taby identified as Taby rather than presenting it as an AgentLight-original character; retain the source artwork terms when redistributing the copied asset pack.

### Candidate status mapping for the first face pack

Validate these clips visually before finalizing the mapping:

| AgentLight state | Candidate Taby animation | Playback behavior |
|---|---|---|
| `IDLE` | `idle_01_loop` | Loop |
| `WORKING` | `working_laptop_loop` (or `working_loop`) | Loop |
| `WAITING` | `waiting_01` followed by `angry_01_loop` | One-shot attention entrance, then loop while waiting persists |
| `ERROR` | `angry_02_loop` | Loop while the error persists |
| `COMPLETED` | `task_completed` | Play once, then return to `idle_01_loop` |

The exact mapping is a product/visual decision. A one-shot must never cause the widget to appear idle while its agent is still `WAITING` or `ERROR`.

## Decisions to preserve

1. **Classic traffic light remains the default.** Existing users and old settings must render exactly as before unless they opt into the face style.
2. **The first style switch is global**, matching the current shared widget-appearance settings. All enabled agent widgets use the chosen visual style. Per-agent style overrides can be considered later.
3. The hardware LED output continues using the existing traffic-light state protocol; this feature changes only virtual windows.
4. Keep asset IDs and catalog metadata in a local manifest. Do not fetch animations at runtime from GitHub or another network location.
5. Only include a curated status pack initially. Add more of the 84 catalog animations after assessing bundle size, rendering cost, and which animations are suitable as persistent status indicators.

## Step-by-step implementation and current status

### Step 1 — Audit and curate artwork

- Make a short visual contact sheet of the candidate idle, working, waiting, error, and completion GIFs and confirm the proposed mapping.
- Record each chosen ID, source path, duration, loop policy, byte length, and SHA-256 from the source catalog.
- Decide whether the first version has one Taby face set or two/three curated face sets (for example, **Classic**, **Calm**, and **Expressive**). Do not expose one-shot story animations as persistent status loops.
- Confirm how the black background, frame corners, transparency, and GIF playback look in the actual Tauri WebView at all supported widget sizes.

**Deliverable:** an approved, small asset list and state mapping.

### Step 2 — Add a local asset pack with provenance

- Copy only approved animation files into an AgentLight-owned static asset directory, keeping stable Taby animation IDs in a manifest rather than exposing source hash names to the UI.
- Store loop policy and `duration_ms` with each asset. Validate the copy against the upstream catalog hashes.
- Add a clear Taby/TRIIIS LABS attribution and source-repository link to `README.md`; include the source artwork terms in packaged notices when redistributing copied artwork. Keep Taby identified as Taby rather than presenting the character as AgentLight-original art.
- Add an asset validation test/script so missing files or changed hashes fail before a release.

**Acceptance:** assets are served locally in dev and packaged MSI/NSIS builds, with no runtime network access.

### Step 3 — Persist visual style compatibly

- Add a serialized `WidgetVisualStyle` setting (`traffic_light` / `animated_face`) with `traffic_light` as the serde and runtime default.
- Add the selected face pack/preset ID as a separate setting. Default it to the first reviewed Taby status pack.
- Extend the existing widget-appearance get/set API and change event rather than replacing the current `size`, `theme`, or `opacity` settings.
- Existing settings files that lack the new fields must load unchanged and continue showing the classic traffic light.

**Acceptance:** selecting a style survives app restart; old settings silently preserve classic rendering.

### Step 4 — Add an isolated face renderer

- Keep the current traffic-light housing/render branch intact. Add a separate `AnimatedFaceWidget` renderer selected only when `visual_style === "animated_face"`.
- Use the existing per-agent `DisplaySnapshot`; map state changes to asset IDs. Change/restart the animation only when the mapped asset changes, not on every snapshot refresh.
- Honor catalog loop policies: sustain loops; after a one-shot, transition to the designated loop/fallback while the state remains active.
- The source GIFs need a 90° rotation for the desired desktop orientation. Apply it once in the shared renderer (never edit each asset), and size face-mode windows to the rotated 456:280 aspect ratio. Use `object-fit: contain` and a dark frame to preserve the opaque AMOLED background. Continue honoring widget size and opacity. Ensure pointer click-to-focus and drag detection, hotzone reveal/hide, and window position work for both renderers.
- Respect `prefers-reduced-motion` with a static poster/reduced-motion mode. If an asset fails to load or is missing, fall back to the classic traffic light rather than an empty widget.

**Acceptance:** there is no visual or behavioral change to the classic traffic-light renderer.

### Step 5 — Add user selection and preview

- Add a clear **Widget style** selector in Virtual Lights: **Traffic light** (selected by default) / **Animated face**.
- When Animated face is selected, show the chosen face set and a small animation preview for each supported state. Make it clear the visual style is shared across agent widgets.
- Offer only reviewed status-compatible face sets in the initial selector; do not confuse one-shot activities such as `basketball_dunk` or `drink_water` with persistent agent states.
- Keep the current traffic-light appearance controls working and make the style-specific size/frame behavior clear.

**Acceptance:** users can switch back to the classic lights at any time without losing widget visibility, position, hotzone, or state configuration.

### Step 6 — Package-size and runtime checks

- Measure total size of the selected assets and resulting installer delta. Avoid eager-decoding every GIF; load only the current face animation and release/replace it on a state transition.
- Verify Tauri packaged asset URLs work in dev and release. Verify transparent window and black AMOLED art render as intended on Windows first; test macOS/Linux if those builds are supported.
- Check multiple enabled agents, hidden agents, hotzone mode, always-on-top, focus-click, size/theme/opacity changes, and system tray show/hide.

### Step 7 — Tests and release checklist

- Unit-test state-to-animation mapping, loop/one-shot fallback, and the legacy default style.
- Add a UI/component test for switching styles and for fallback when an asset fails.
- Run `npm run build`, `npm run lint`, Rust tests, asset-integrity checks, and a packaged Tauri build.
- Manually verify: fresh settings show the traffic light; opting into the face shows Taby animations; switching back restores the unchanged traffic light; face mode continues following each agent's own status.
- Include Taby/TRIIIS LABS attribution in `README.md` and release materials, and review the final screen to ensure it is visibly Taby art, not rebranded as another character.

## Current implementation status

Implemented in the current branch:

- The classic traffic light remains the default and keeps its existing renderer; animated face mode and three Taby face sets are persisted through the shared widget-appearance setting. The source GIF orientation is corrected once in CSS with a 90° rotation, and face-mode windows use a landscape aspect ratio.
- A curated 12-GIF subset plus a static WebP poster per animation is copied locally under `public/assets/taby/`; the source catalog metadata and hashes are checked by `npm run test:assets`.
- IDLE/WORKING/WAITING/ERROR/COMPLETED mappings use loop and one-shot metadata, including a persistent attention fallback. Failed image loads fall back to the classic light; reduced-motion preference shows posters instead of animated GIFs.
- README attribution and the source artwork terms are included.

Still required before calling this visually shipped: manually inspect the widget in Tauri dev and a packaged Windows build at all sizes, verify GIF black backgrounds and face crop/scale, test preference persistence and reduced-motion behavior, and confirm the first face set visually matches the intended AgentLight tone.

## Risks and open decisions

- **Art direction:** review the supplied GIFs in the actual widget before selecting status mappings. The one-shot `waiting_01` is about 12.9 seconds; decide whether that is appropriate before switching to a persistent attention loop.
- **Aspect ratio:** the source art is 280×456 portrait and is rotated to a 456×280 landscape display. Face-mode dimensions now follow that rotated ratio; verify it at each widget size and DPI setting.
- **GIF playback policy:** confirm that each candidate's encoded GIF behavior agrees with `catalog.json` and implement the catalog's loop/one-shot behavior explicitly.
- **Bundle size:** start with the small status subset, then decide whether the wider animation library warrants a later downloadable/optional pack.
- **Attribution/branding:** credit Taby and TRIIIS LABS in `README.md` (and packaged notices when distributing the artwork); do not rebrand the character.
- **Settings scope:** initial recommendation is one shared visual style, matching current shared widget appearance. Ask whether users eventually need a different style per agent before adding per-agent preference state.

## Definition of done

A clean install starts with the existing traffic light and it looks/behaves unchanged. A user can opt into a local Taby animated-face widget from settings; it follows agent state transitions, remains visible for persistent attention states, respects reduced-motion and existing window/hotzone behavior, and can be switched back to traffic lights. The selected style persists across restarts, assets are verified and packaged locally, and Taby artwork attribution/terms ship with the app.
