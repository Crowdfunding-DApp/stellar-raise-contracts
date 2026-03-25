# React Submit Button Component States

Addresses [GitHub Issue #359](https://github.com/Crowdfunding-DApp/stellar-raise-contracts/issues/359).

A typed, accessible React submit button with a deterministic state machine, secure label handling, and double-submit prevention for crowdfunding transaction flows.

---

## Files

| File | Purpose |
|------|---------|
| `react_submit_button.tsx` | Component implementation and exported helpers |
| `react_submit_button.test.tsx` | Test suite (≥ 95% coverage) |
| `react_submit_button.md` | This document |

---

## State Model

The button moves through a strict state machine:

```
idle ──click──► submitting ──resolve──► success ──► idle
                            └──reject──► error   ──► idle
any state ──────────────────────────────────────► disabled
```

| State | Visual | Interaction | `aria-busy` |
|-------|--------|-------------|-------------|
| `idle` | Indigo | Clickable | `false` |
| `submitting` | Light indigo | Blocked | `true` |
| `success` | Green | Blocked | `false` |
| `error` | Red | Clickable (retry) | `false` |
| `disabled` | Grey | Blocked | `false` |

Transitions are validated against an allowlist when `strictTransitions` is enabled (default: `true`). Invalid transitions fall back to the previous valid state.

---

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `state` | `SubmitButtonState` | required | Current button state |
| `previousState` | `SubmitButtonState` | — | Used for transition validation in strict mode |
| `strictTransitions` | `boolean` | `true` | Reject invalid state jumps |
| `labels` | `SubmitButtonLabels` | — | Per-state label overrides |
| `onClick` | `(e) => void \| Promise<void>` | — | Click handler; blocked while submitting/disabled |
| `className` | `string` | — | Additional CSS class names |
| `id` | `string` | — | HTML id attribute |
| `type` | `"button" \| "submit"` | `"button"` | HTML button type |
| `disabled` | `boolean` | — | External disabled flag |

---

## Usage

```tsx
import ReactSubmitButton from "../components/react_submit_button";

<ReactSubmitButton
  state="idle"
  previousState="idle"
  strictTransitions
  type="submit"
  labels={{ idle: "Fund Campaign", submitting: "Funding..." }}
  onClick={handleFund}
/>
```

---

## Exported Helper Functions

### `normalizeSubmitButtonLabel(candidate, fallback)`
- **@notice** Sanitizes a raw label value before rendering.
- **@dev** Rejects non-strings, strips control characters (U+0000–U+001F, U+007F), normalizes whitespace, and truncates to 80 characters.
- **@security** Prevents blank UI states and layout abuse via oversized labels.

### `resolveSubmitButtonLabel(state, labels?)`
- **@notice** Returns a safe, non-empty display label for the given state.
- **@dev** Delegates to `normalizeSubmitButtonLabel`; falls back to built-in defaults.

### `isValidSubmitButtonStateTransition(previousState, nextState)`
- **@notice** Validates whether a transition between two states is permitted.
- **@dev** Same-state transitions are always valid (idempotent updates).

### `resolveSafeSubmitButtonState(state, previousState?, strictTransitions?)`
- **@notice** Returns the effective state, blocking invalid transitions in strict mode.
- **@security** Prevents race-condition state jumps (e.g. `idle → success`).

### `isSubmitButtonInteractionBlocked(state, disabled?, isLocallySubmitting?)`
- **@notice** Returns `true` when clicks should be suppressed.
- **@security** Prevents duplicate blockchain transactions on rapid clicks.

### `isSubmitButtonBusy(state, isLocallySubmitting?)`
- **@notice** Returns `true` when the button should signal a loading/busy state.
- **@dev** Drives the `aria-busy` attribute for assistive technology.

---

## Security Assumptions and Safeguards

| Assumption | Safeguard |
|------------|-----------|
| Labels may originate from untrusted sources (CMS, API, operator config) | `normalizeSubmitButtonLabel` strips control chars, rejects non-strings, caps length |
| Consumers may pass markup-like strings as labels | Labels are rendered as React text nodes — no `dangerouslySetInnerHTML` path exists |
| Users may click rapidly during async operations | `isLocallySubmitting` flag blocks re-entry before parent state updates |
| Parent state may jump illegally under race conditions | `resolveSafeSubmitButtonState` enforces the transition allowlist in strict mode |

---

## Test Coverage

Run with:

```bash
npm test -- --testPathPattern=react_submit_button --coverage
```

The suite covers:

- `normalizeSubmitButtonLabel` — non-string fallback, empty/whitespace fallback, control-char stripping, truncation, exact-length boundary
- `resolveSubmitButtonLabel` — all default labels, custom overrides, hostile markup assumption
- `isValidSubmitButtonStateTransition` — all valid forward transitions, same-state idempotency, all invalid transitions
- `resolveSafeSubmitButtonState` — valid/invalid strict transitions, non-strict mode, missing previousState
- `isSubmitButtonInteractionBlocked` — submitting/disabled states, explicit flag, local in-flight flag, active states
- `isSubmitButtonBusy` — submitting state, local in-flight flag, all non-busy states
