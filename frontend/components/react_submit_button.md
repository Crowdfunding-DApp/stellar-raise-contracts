# ReactSubmitButton

Refactored React submit button component with a deterministic state machine,
double-submit prevention, and full accessibility support for crowdfunding
transaction flows (contribute, withdraw, refund).

---

## Files

| File | Purpose |
|------|---------|
| `react_submit_button.tsx` | Component implementation |
| `react_submit_button_types.ts` | Types, STATE_CONFIG, pure helper functions |
| `react_submit_button.test.tsx` | Test suite (≥ 95 % coverage) |
| `react_submit_button.md` | This document |

---

## State Machine

```
idle ──click──► submitting ──resolve──► success ──(resetDelay)──► idle
                           └──reject──► error   ──(resetDelay)──► idle
Any state ──disabled=true──► disabled
```

| State | Visual | `disabled` | `aria-busy` | Clickable |
|-------|--------|-----------|-------------|-----------|
| `idle` | Indigo | No | No | Yes |
| `submitting` | Light indigo | Yes | Yes | No |
| `success` | Green + ✓ | Yes | No | No |
| `error` | Red + retry | No | No | Yes (retry) |
| `disabled` | Grey, 60 % opacity | Yes | No | No |

---

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `label` | `string` | required | Text shown in idle and disabled states |
| `onClick` | `() => Promise<void>` | required | Async handler; rejection triggers error state |
| `disabled` | `boolean` | `false` | External disabled flag (maps to disabled state) |
| `resetDelay` | `number` | `2500` | ms before auto-reset from success/error to idle |
| `type` | `"submit" \| "button" \| "reset"` | `"submit"` | HTML button type |
| `style` | `React.CSSProperties` | — | Extra inline styles merged onto the button |
| `data-testid` | `string` | — | Test selector |

---

## Usage

```tsx
import ReactSubmitButton from "../components/react_submit_button";

function ContributeForm() {
  return (
    <ReactSubmitButton
      label="Contribute"
      onClick={async () => {
        await invokeContractContribute(/* ... */);
      }}
    />
  );
}
```

### With all options

```tsx
<ReactSubmitButton
  label="Fund Campaign"
  onClick={handleContribute}
  disabled={!walletConnected}
  resetDelay={3000}
  type="button"
  data-testid="contribute-btn"
/>
```

---

## NatSpec-Style Reference

### `ReactSubmitButton`
- **@notice** Accessible submit button with idle / submitting / success / error / disabled states.
- **@param** `label` — Text shown in idle and disabled states.
- **@param** `onClick` — Async handler; must return `Promise<void>`. Rejection triggers error state.
- **@param** `disabled` — When `true`, maps to the `disabled` state and blocks interaction.
- **@param** `resetDelay` — Milliseconds before auto-reset. Default `2500`. Clamped to `≥ 0`.
- **@security** Clicks are ignored in non-idle/non-error states (double-submit protection).
- **@security** Timer is cleaned up on unmount (memory-leak protection).

### `STATE_CONFIG`
- **@notice** Centralised visual and accessibility configuration for each button state.
- **@dev** All colours are hardcoded hex values — no dynamic CSS injection from user input.

### `isValidStateTransition(from, to)`
- **@notice** Returns `true` if the transition from `from` to `to` is allowed.
- **@dev** Same-state transitions are always allowed (idempotent updates).

### `isInteractionBlocked(state, disabled?)`
- **@notice** Returns `true` when the button should be non-interactive.

### `isBusy(state)`
- **@notice** Returns `true` when `aria-busy` should be set (submitting only).

---

## Security Assumptions

1. **No HTML injection** — `label` and all state labels are rendered as React text nodes,
   never via `dangerouslySetInnerHTML`. XSS via the label prop is not possible.
2. **Double-submit prevention** — Clicks are silently ignored in `submitting`, `success`,
   and `disabled` states, preventing duplicate blockchain transactions.
3. **No dynamic CSS injection** — All background colours are sourced from `STATE_CONFIG`
   constants. No user-supplied strings are interpolated into CSS values.
4. **Timer cleanup** — The reset timer is cleared on unmount via `useEffect` cleanup,
   preventing state updates on unmounted components and potential memory leaks.
5. **Negative resetDelay clamped** — `Math.max(0, resetDelay)` ensures a negative value
   cannot cause unexpected behaviour.

---

## Test Coverage

Run with:

```bash
npm test -- --testPathPattern=react_submit_button --coverage
```

| Category | Tests |
|----------|-------|
| STATE_CONFIG completeness | 12 |
| isValidStateTransition | 14 |
| isInteractionBlocked | 7 |
| isBusy | 5 |
| Component rendering (idle) | 7 |
| Component rendering (disabled) | 4 |
| Click handler — success path | 2 |
| Click handler — error path | 4 |
| Double-submit prevention | 3 |
| Auto-reset timer | 4 |
| Accessibility | 7 |
| Security edge cases | 5 |
| Style merging | 2 |

Target: ≥ 95 % line coverage.
