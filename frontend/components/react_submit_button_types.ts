/**
 * @title SubmitButton — Types and State Configuration
 * @notice Pure TypeScript exports with no React dependency.
 *         Imported by both the component and the test suite.
 *
 * @security All colour values are hardcoded constants — no dynamic CSS injection.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

/**
 * @notice All possible visual/interaction states of the submit button.
 * @dev State transitions:
 *      idle → submitting → success | error → idle (auto-reset)
 *      Any state → disabled (via external prop)
 */
export type ButtonState = "idle" | "submitting" | "success" | "error" | "disabled";

/**
 * @notice Props accepted by the ReactSubmitButton component.
 */
export interface SubmitButtonProps {
  /** Text shown in the idle and disabled states. */
  label: string;
  /** Async click handler; rejection triggers the error state. */
  onClick: () => Promise<void>;
  /** Externally controlled disabled flag (maps to the disabled state). */
  disabled?: boolean;
  /** Milliseconds before auto-resetting from success/error back to idle. Default: 2500. */
  resetDelay?: number;
  /** Override the button's HTML type attribute. Default: "submit". */
  type?: "submit" | "button" | "reset";
  /** Additional inline styles merged onto the button element. */
  style?: React.CSSProperties;
  /** Optional test id for targeting in tests. */
  "data-testid"?: string;
}

// ── State configuration ───────────────────────────────────────────────────────

/**
 * @notice Visual and accessibility configuration for each button state.
 * @dev Centralising colours here makes security review straightforward —
 *      no dynamic style injection from user input.
 */
export const STATE_CONFIG: Record<
  ButtonState,
  { label: string; backgroundColor: string; cursor: string; ariaLabel: string }
> = {
  idle: {
    label: "",
    backgroundColor: "#4f46e5",
    cursor: "pointer",
    ariaLabel: "",
  },
  submitting: {
    label: "Processing\u2026",
    backgroundColor: "#6366f1",
    cursor: "not-allowed",
    ariaLabel: "Processing, please wait",
  },
  success: {
    label: "Success \u2713",
    backgroundColor: "#16a34a",
    cursor: "default",
    ariaLabel: "Action completed successfully",
  },
  error: {
    label: "Failed \u2014 retry",
    backgroundColor: "#dc2626",
    cursor: "pointer",
    ariaLabel: "Action failed, click to retry",
  },
  disabled: {
    label: "",
    backgroundColor: "#9ca3af",
    cursor: "not-allowed",
    ariaLabel: "Button disabled",
  },
};

// ── Allowed state transitions ─────────────────────────────────────────────────

/**
 * @notice Defines valid next states for each current state.
 * @dev Used by isValidStateTransition to guard against invalid jumps.
 */
export const ALLOWED_TRANSITIONS: Record<ButtonState, ButtonState[]> = {
  idle: ["submitting", "disabled"],
  submitting: ["success", "error", "disabled"],
  success: ["idle", "disabled"],
  error: ["idle", "submitting", "disabled"],
  disabled: ["idle"],
};

// ── Pure helper functions ─────────────────────────────────────────────────────

/**
 * @notice Returns true if the transition from `from` to `to` is allowed.
 * @dev Same-state transitions are always allowed (idempotent updates).
 */
export function isValidStateTransition(from: ButtonState, to: ButtonState): boolean {
  if (from === to) return true;
  return ALLOWED_TRANSITIONS[from].includes(to);
}

/**
 * @notice Returns true when the button should be non-interactive.
 */
export function isInteractionBlocked(state: ButtonState, disabled = false): boolean {
  return Boolean(disabled) || state === "submitting" || state === "success" || state === "disabled";
}

/**
 * @notice Returns true when aria-busy should be set.
 */
export function isBusy(state: ButtonState): boolean {
  return state === "submitting";
}
