# UI Guidelines

PulseRing is a quiet status utility, not a dashboard. The interface should feel calm, compact and predictable across the settings window, tray tooltip and floating bar.

## Visual System

- Use the shared tokens in `ui/styles.css` for color, spacing, radius, type size, shadow and motion.
- Keep controls at the same visual density: 34px minimum height, rounded corners and visible focus states.
- Neutral panels with one blue accent for controls. Color otherwise means status only.
- Status palette: green = normal, amber = elevated, red = critical. The tray icon (`level_rgb` in `src-tauri/src/ui/tray.rs`) and CSS (`--ok-fill`, `--warn-fill`, `--crit-fill`) must stay in sync.
- Use `*-fill` tokens for rings, meters and dots; use `*-text` tokens for colored text so it stays readable on light surfaces.
- Transparent windows (floating bar, tooltip) must keep shadows inside the window padding; nothing outside the card may paint.
- Floating-bar opacity applies to the background only, never to text.
- Use tabular numbers for live metrics so values do not visually jump every second.

## Type

- Settings headings use the largest type in the app.
- Section headings describe the task area, not the implementation.
- Field labels should be short nouns or verb phrases.
- Field descriptions should explain user impact in one sentence.
- Status surfaces use compact labels: `CPU`, `内存`, `GPU`, `网络`.

## Layout

- Settings sections follow this structure: heading, short description, body controls.
- Do not expose dormant configuration fields unless the app supports them end to end.
- Keep platform-specific controls platform-specific. Windows floating-window options should not appear on macOS unless the feature works there.
- Tooltip rows and floating-bar metric pills should use the same metric names and ordering.

## Motion

- Use short entrance motion only: settings page in around 260ms, status surfaces around 150-180ms.
- Do not animate metric changes every second beyond short ring/meter transitions. The utility should feel stable while values update.
- The only looping animation is the soft glow on the status ring at the critical level.
- Live surfaces patch the DOM in place (`ui/components/morph.js`) instead of replacing it, so hover and clicks survive updates.
- Respect `prefers-reduced-motion`.

## Metric Display

- Ordering is always CPU, memory, GPU, network.
- Percent values are rounded whole numbers.
- Network values use automatic units and compact labels on small surfaces.
- Missing optional metrics should show `N/A` rather than disappearing, unless a future design intentionally reserves less space.

## Copy

- Chinese UI copy should be direct and practical.
- Avoid technical implementation terms in user-facing descriptions.
- Keep destructive or app-level actions in the footer.
