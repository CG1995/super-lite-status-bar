# UI Guidelines

PulseRing is a quiet status utility, not a dashboard. The interface should feel calm, compact and predictable across the settings window, tray tooltip and floating bar.

## Visual System

- Use the shared tokens in `ui/styles.css` for color, spacing, radius, type size, shadow and motion.
- Keep controls at the same visual density: 34px minimum height, rounded corners and visible focus states.
- Prefer warm neutral panels with subtle accent color. Avoid adding unrelated brand colors unless they map to a metric state.
- Use tabular numbers for live metrics so values do not visually jump every second.

## Type

- Settings headings use the largest type in the app.
- Section headings describe the task area, not the implementation.
- Field labels should be short nouns or verb phrases.
- Field descriptions should explain user impact in one sentence.
- Status surfaces use compact uppercase labels: `CPU`, `MEM`, `GPU`, `NET`.

## Layout

- Settings sections follow this structure: heading, short description, body controls.
- Do not expose dormant configuration fields unless the app supports them end to end.
- Keep platform-specific controls platform-specific. Windows floating-window options should not appear on macOS unless the feature works there.
- Tooltip rows and floating-bar metric pills should use the same metric names and ordering.

## Motion

- Use short entrance motion only: settings page in around 260ms, status surfaces around 150-180ms.
- Do not animate metric changes every second. The utility should feel stable while values update.
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
