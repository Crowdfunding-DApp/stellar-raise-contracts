/**
 * @title ReactSubmitButton — Shared Types and State Configuration
 * @notice Pure TypeScript exports with no React dependency.
 *         Imported by both the component and the test suite.
 *
 * @security All colour values are hardcoded constants — no dynamic CSS injection
 *           from user input is possible.
 */

// Re-export the canonical state type so consumers can import from one place.
export type { SubmitButtonState, SubmitButtonLabels, ReactSubmitButtonProps } from "./react_submit_button";
export { ALLOWED_TRANSITIONS } from "./react_submit_button";

// ── State configuration ───────────────────────────────────────────────────────

import type { SubmitButtonState } from "./react_submit_button";

/**
 * @notice Visual and accessibility configuration for each button state.
 * @dev Centralising colours here makes security review straightforward —
 *      no dynamic style injection from user input.
 */
export const STATE_CONFIG: Record<
  SubmitButtonState,
  { label: string; backgroundColor: string; cursor: string; ariaLabel: string }
> = {
  idle: {
    label: "Submit",
    backgroundColor: "#4f46e5",
    cursor: "pointer",
    ariaLabel: "Submit",
  },
  submitting: {
    label: "Submitting\u2026",
    backgroundColor: "#6366f1",
    cursor: "not-allowed",
    ariaLabel: "Submitting, please wait",
  },
  success: {
    label: "Submitted \u2713",
    backgroundColor: "#16a34a",
    cursor: "default",
    ariaLabel: "Action completed successfully",
  },
  error: {
    label: "Try Again",
    backgroundColor: "#dc2626",
    cursor: "pointer",
    ariaLabel: "Action failed, click to retry",
  },
  disabled: {
    label: "Submit Disabled",
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
