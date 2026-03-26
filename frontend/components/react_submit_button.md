# ReactSubmitButton

A typed, accessible React submit button driven by an explicit state machine.
Designed for transaction flows where duplicate submissions must be prevented.

---

## Files

| File | Purpose |
|------|---------|
| `react_submit_button.tsx` | Component and all exported utilities |
| `react_submit_button.test.tsx` | Full test suite (≥ 95% coverage) |
| `react_submit_button.md` | This document |

---

## States

```
idle ──click──► submitting ──resolve──► success
                           └──reject──► error ──retry──► submitting
                                                └──reset──► idle
any ──────────────────────────────────────────► disabled
```

| State | Label (default) | Clickable | `disabled` attr | `aria-busy` |
|-------|----------------|-----------|-----------------|-------------|
| `idle` | Submit | ✓ | No | No |
| `submitting` | Submitting... | ✗ | Yes | Yes |
| `success` | Submitted | ✓ | No | No |
| `error` | Try Again | ✓ | No | No |
| `disabled` | Submit Disabled | ✗ | Yes | No |

---

## Usage

```tsx
import ReactSubmitButton from "./react_submit_button";

<ReactSubmitButton
  state="idle"
  onClick={async (e) => {
    await submitTransaction();
  }}
/>
```

### With all options

```tsx
<ReactSubmitButton
  state={buttonState}
  previousState={previousButtonState}
  strictTransitions
  labels={{ idle: "Fund Campaign", submitting: "Funding..." }}
  onClick={handleSubmit}
  disabled={!walletConnected}
  type="submit"
  id="fund-btn"
  className="btn-primary"
/>
```

---

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `state` | `SubmitButtonState` | required | Drives label, style, and interaction |
| `previousState` | `SubmitButtonState` | — | Enables strict transition guard |
| `strictTransitions` | `boolean` | `true` | Blocks invalid transitions when true |
| `labels` | `SubmitButtonLabels` | — | Per-state label overrides |
| `onClick` | `(e: MouseEvent) => void \| Promise<void>` | — | Click handler; blocked while submitting |
| `disabled` | `boolean` | `false` | External disabled flag |
| `type` | `"button" \| "submit"` | `"button"` | HTML button type |
| `id` | `string` | — | Element id |
| `className` | `string` | — | CSS class |

---

## Exported Utilities

### `normalizeSubmitButtonLabel(candidate, fallback)`
- **@notice** Strips control characters (`U+0000–U+001F`, `U+007F`), normalizes whitespace, and truncates to 80 characters.
- **@param** `candidate` — Any value; non-strings return `fallback`.
- **@param** `fallback` — Returned when candidate is unusable.
- **@returns** A safe, non-empty string.

### `resolveSubmitButtonLabel(state, labels?)`
- **@notice** Returns the label for `state`, applying `normalizeSubmitButtonLabel` to any override.
- **@returns** Normalized custom label, or the built-in default.

### `isValidSubmitButtonStateTransition(previousState, nextState)`
- **@notice** Returns `true` when the transition is permitted by the state machine.
- **@dev** Same-state transitions are always allowed (idempotent updates).

### `resolveSafeSubmitButtonState(state, previousState?, strictTransitions?)`
- **@notice** Returns `state` if the transition is valid, otherwise `previousState`.
- **@dev** When `strictTransitions` is `false` or `previousState` is absent, `state` is returned unconditionally.

### `isSubmitButtonInteractionBlocked(state, disabled?, isLocallySubmitting?)`
- **@notice** Returns `true` when the button should not respond to clicks.
- **@security** Prevents duplicate submissions during in-flight async handlers.

### `isSubmitButtonBusy(state, isLocallySubmitting?)`
- **@notice** Returns `true` when `aria-busy` should be set.

---

## Security Assumptions

### Double-submit prevention
`onClick` is wrapped in a local in-flight guard (`isLocallySubmitting`). While the handler is executing, all further clicks are silently ignored — even if the parent has not yet updated `state` to `submitting`. This closes the race window between click and state propagation.

### No HTML injection
All labels are rendered as React text nodes. `dangerouslySetInnerHTML` is never used. Markup-like strings (e.g. `<script>`) are inert.

### No dynamic CSS injection
All colours and cursors come from `STATE_STYLE_MAP`, a hardcoded constant. No user-supplied string is interpolated into a CSS value.

### Label length bound
Labels are capped at 80 characters. Oversized strings are truncated with `...` to prevent layout overflow attacks.

### Control character stripping
`U+0000–U+001F` and `U+007F` are replaced with a space before rendering, preventing invisible or misleading label content.

---

## Running Tests

```bash
npm test -- --testPathPattern=react_submit_button --coverage
```

Coverage targets: statements ≥ 95%, branches ≥ 95%, functions 100%, lines ≥ 95%.
