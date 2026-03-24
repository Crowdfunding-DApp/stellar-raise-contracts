# ReactSubmitButton

A typed React submit button with a strict state machine, safe label handling, double-submit prevention, and ARIA accessibility semantics.
# SubmitButton Component

Addresses [GitHub Issue #359](https://github.com/Crowdfunding-DApp/stellar-raise-contracts/issues/359).

A robust, accessible React submit button with full state management for crowdfunding transaction flows.

---

## Files

| File | Purpose |
|------|---------|
| `react_submit_button.tsx` | Component implementation |
| `react_submit_button.test.tsx` | Test suite (≥ 95% coverage) |
| `react_submit_button.md` | This document |

---

## States

| State        | Description                                      | Clickable |
|--------------|--------------------------------------------------|-----------|
| `idle`       | Default — ready to submit                        | ✅        |
| `submitting` | Async action in-flight; blocks interaction       | ❌        |
| `success`    | Action confirmed                                 | ✅        |
| `error`      | Action failed; user can retry                    | ✅        |
| `disabled`   | Externally locked (deadline passed, goal met…)   | ❌        |

### Allowed transitions

```
idle        → submitting | disabled
submitting  → success | error | disabled
success     → idle | disabled
error       → idle | submitting | disabled
disabled    → idle
```

Same-state updates are always allowed (idempotent).

---

## Props

| Prop                | Type                                              | Default      | Description                                              |
|---------------------|---------------------------------------------------|--------------|----------------------------------------------------------|
| `state`             | `SubmitButtonState`                               | —            | Current button state (required)                          |
| `previousState`     | `SubmitButtonState`                               | `undefined`  | Previous state for strict transition validation          |
| `strictTransitions` | `boolean`                                         | `true`       | Falls back to `previousState` on invalid transitions     |
| `labels`            | `SubmitButtonLabels`                              | `undefined`  | Per-state label overrides                                |
| `onClick`           | `(e: MouseEvent) => void \| Promise<void>`        | `undefined`  | Click handler; blocked while submitting/disabled         |
| `className`         | `string`                                          | `undefined`  | Additional CSS class                                     |
| `id`                | `string`                                          | `undefined`  | HTML `id` attribute                                      |
| `type`              | `"button" \| "submit" \| "reset"`                 | `"button"`   | HTML button type                                         |
| `disabled`          | `boolean`                                         | `undefined`  | External disabled override                               |

---

## Default labels

| State        | Label             |
|--------------|-------------------|
| `idle`       | `Submit`          |
| `submitting` | `Submitting...`   |
| `success`    | `Submitted`       |
| `error`      | `Try Again`       |
| `disabled`   | `Submit Disabled` |
The button moves through a deterministic state machine:

```
idle ──click──► loading ──resolve──► success ──resetDelay──► idle
                        └──reject──► error   ──resetDelay──► idle
```

| State | Visual | Interaction | Native `disabled` |
|-------|--------|-------------|-------------------|
| `idle` | Indigo | Clickable | No |
| `loading` | Light indigo + spinner | Blocked | Yes |
| `success` | Green + ✓ | Blocked | Yes |
| `error` | Red + retry label | Clickable (retry) | No |
| `disabled` | Grey, 60% opacity | Blocked | Yes |

---

## Usage

```tsx
import ReactSubmitButton from "./react_submit_button";

// Basic
<ReactSubmitButton state="idle" onClick={handleSubmit} />

// With custom labels
<ReactSubmitButton
  state={txState}
  previousState={prevTxState}
  labels={{ idle: "Fund Campaign", submitting: "Funding...", success: "Funded!" }}
  onClick={handleContribute}
/>

// Externally disabled (e.g. campaign deadline passed)
<ReactSubmitButton state="disabled" labels={{ disabled: "Campaign Ended" }} />
import SubmitButton from "../components/react_submit_button";

<SubmitButton
  label="Fund Campaign"
  onClick={async () => {
    await submitTransaction();
  }}
/>
```

### With all options

```tsx
<SubmitButton
  label="Contribute"
  onClick={handleContribute}
  disabled={!walletConnected}
  resetDelay={3000}
  type="button"
  data-testid="contribute-btn"
/>
```

---

## Exported helpers

All pure functions are exported for independent unit testing.

| Function                              | Purpose                                                        |
|---------------------------------------|----------------------------------------------------------------|
| `normalizeSubmitButtonLabel`          | Sanitizes a label: strips control chars, truncates to 80 chars |
| `resolveSubmitButtonLabel`            | Returns the safe label for a given state                       |
| `isValidSubmitButtonStateTransition`  | Validates a `from → to` state transition                       |
| `resolveSafeSubmitButtonState`        | Enforces strict transitions, falls back to `previousState`     |
| `isSubmitButtonInteractionBlocked`    | Returns `true` when clicks must be suppressed                  |
| `isSubmitButtonBusy`                  | Returns `true` when `aria-busy` should be set                  |
| `ALLOWED_TRANSITIONS`                 | Transition map (shared by component and tests)                 |

---

## Security assumptions

- **No `dangerouslySetInnerHTML`** — labels are rendered as React text nodes only.
- **Label sanitization** — control characters (`U+0000–U+001F`, `U+007F`) are stripped; labels are truncated to 80 characters to prevent layout abuse.
- **Double-submit prevention** — an internal `isLocallySubmitting` flag blocks re-entry while an async `onClick` is in-flight, preventing duplicate blockchain transactions.
- **Hardcoded styles** — all CSS values are compile-time constants; no dynamic style injection from user input.
- **Input validation is the caller's responsibility** — the component surfaces state only; it never submits data itself.

---

## Accessibility

- `aria-live="polite"` — state label changes are announced to screen readers.
- `aria-busy` — set to `true` while submitting.
- `aria-label` — always set to the resolved, sanitized label.
- `disabled` — set on the HTML element when interaction is blocked, preventing keyboard activation.

---

## Tests

```
frontend/components/react_submit_button.test.tsx
```

51 tests covering:
- Label normalization and sanitization edge cases
- Default and custom label resolution per state
- State transition validation (allowed, blocked, idempotent)
- Strict transition enforcement and fallback
- Interaction blocking (submitting, disabled, external flag, local in-flight)
- `aria-busy` / `aria-live` / `aria-label` attributes
- Click handler: idle, error (retry), blocked states, async, rejected promise
- Rendering: element type, `data-state`, `type`, `className`, `id`
## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `label` | `string` | required | Button text in idle/disabled states |
| `onClick` | `() => Promise<void>` | required | Async handler; rejection triggers error state |
| `disabled` | `boolean` | `false` | External disabled flag |
| `resetDelay` | `number` | `2500` | ms before auto-reset from success/error |
| `type` | `"submit" \| "button" \| "reset"` | `"submit"` | HTML button type |
| `style` | `React.CSSProperties` | — | Extra inline styles |
| `data-testid` | `string` | — | Test selector |

---

## Security Assumptions

### Double-submit prevention
Clicks are silently ignored in `loading`, `success`, and `disabled` states. This prevents duplicate blockchain transactions (double-spend) when a user clicks repeatedly while a transaction is in-flight.

### No HTML injection
The `label` prop and all state labels are rendered as React text nodes, never via `dangerouslySetInnerHTML`. XSS via the label prop is not possible.

### No user-controlled styles
Background colours and cursors are sourced exclusively from the `STATE_CONFIG` constant. No user-supplied strings are interpolated into CSS values.

### Timer cleanup
The reset timer is cleared on component unmount via a `useEffect` cleanup function, preventing state updates on unmounted components and potential memory leaks.

### Negative `resetDelay` clamped
`Math.max(0, resetDelay)` ensures a negative value cannot cause unexpected behaviour.

---

## NatSpec-style Reference

### `SubmitButton`
- **@notice** Accessible submit button with idle / loading / success / error / disabled states.
- **@param** `label` — Text shown in idle and disabled states.
- **@param** `onClick` — Async handler; must return `Promise<void>`. Rejection triggers error state.
- **@param** `disabled` — When `true`, maps to the `disabled` state and blocks interaction.
- **@param** `resetDelay` — Milliseconds before auto-reset. Default `2500`. Clamped to `≥ 0`.
- **@security** Clicks are ignored in non-idle/non-error states (double-submit protection).
- **@security** Timer is cleaned up on unmount (memory-leak protection).

### `STATE_CONFIG`
- **@notice** Centralised visual configuration for each button state.
- **@dev** All colours are hardcoded hex values — no dynamic CSS injection.

### `ButtonState`
- **@notice** Union type: `"idle" | "loading" | "success" | "error" | "disabled"`.

---

## Test Coverage

Run with:

```bash
npm test -- --testPathPattern=react_submit_button --coverage
```

The suite covers:

- `STATE_CONFIG` completeness and correctness (14 tests)
- `ButtonState` type validation (3 tests)
- `SubmitButtonProps` interface (6 tests)
- State transition logic — all paths (8 tests)
- Security: double-submit prevention (3 tests)
- Accessibility attributes (5 tests)
- Display label logic including XSS edge case (6 tests)
- `resetDelay` edge cases (3 tests)
- Style configuration (3 tests)
- Integration: full lifecycle simulations (2 tests)
