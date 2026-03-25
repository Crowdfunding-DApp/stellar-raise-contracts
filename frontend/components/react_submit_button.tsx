import React, { useMemo, useState } from "react";

/**
 * @title React Submit Button Component States
 * @notice Typed submit button with deterministic state machine, secure label handling,
 *         double-submit prevention, and accessible ARIA semantics.
 * @dev    Uses strict union types and a transition allowlist to prevent invalid state jumps.
 *         Labels from untrusted sources are normalized before rendering.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

/**
 * @notice All supported visual/interaction states of the submit button.
 * @dev    Transitions between states are validated by ALLOWED_STATE_TRANSITIONS.
 */
export type SubmitButtonState = "idle" | "submitting" | "success" | "error" | "disabled";

/**
 * @notice Optional per-state label overrides.
 * @dev    Values are normalized (trimmed, control-chars stripped, length-capped) before use.
 */
export interface SubmitButtonLabels {
  idle?: string;
  submitting?: string;
  success?: string;
  error?: string;
  disabled?: string;
}

/**
 * @notice Props accepted by the ReactSubmitButton component.
 * @param state            Current button state from the parent.
 * @param previousState    Previous state used for transition validation in strict mode.
 * @param strictTransitions When true, invalid state transitions fall back to previousState.
 * @param labels           Optional label overrides for each state.
 * @param onClick          Async-safe click handler; blocked while submitting or disabled.
 * @param className        Additional CSS class names.
 * @param id               HTML id attribute.
 * @param type             HTML button type. Defaults to "button".
 * @param disabled         External disabled flag; merged with state-derived disabled logic.
 */
export interface ReactSubmitButtonProps {
  state: SubmitButtonState;
  previousState?: SubmitButtonState;
  strictTransitions?: boolean;
  labels?: SubmitButtonLabels;
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void | Promise<void>;
  className?: string;
  id?: string;
  type?: "button" | "submit";
  disabled?: boolean;
}

// ── Constants ─────────────────────────────────────────────────────────────────

/** @notice Fallback labels used when no override is provided or the override is invalid. */
const DEFAULT_LABELS: Required<SubmitButtonLabels> = {
  idle: "Submit",
  submitting: "Submitting...",
  success: "Submitted",
  error: "Try Again",
  disabled: "Submit Disabled",
};

/** @notice Maximum allowed label length. Labels exceeding this are truncated with an ellipsis. */
const MAX_LABEL_LENGTH = 80;

/** @notice Matches ASCII control characters (0x00–0x1F, 0x7F) that should not appear in labels. */
const CONTROL_CHARACTER_REGEX = /[\u0000-\u001F\u007F]/g;

/**
 * @notice Allowlist of valid state transitions.
 * @dev    Only transitions listed here are permitted when strictTransitions is enabled.
 *         Same-state transitions are always allowed (idempotent updates).
 */
const ALLOWED_STATE_TRANSITIONS: Record<SubmitButtonState, SubmitButtonState[]> = {
  idle: ["submitting", "disabled"],
  submitting: ["success", "error", "disabled"],
  success: ["idle", "disabled"],
  error: ["idle", "submitting", "disabled"],
  disabled: ["idle"],
};

/** @notice Base inline styles applied to every button instance. */
const BASE_STYLE: React.CSSProperties = {
  minHeight: "44px",
  minWidth: "120px",
  borderRadius: "8px",
  border: "1px solid #4f46e5",
  padding: "0.5rem 1rem",
  color: "#ffffff",
  fontWeight: 600,
  cursor: "pointer",
  transition: "opacity 0.2s ease",
  backgroundColor: "#4f46e5",
};

/** @notice Per-state style overrides merged on top of BASE_STYLE. */
const STATE_STYLE_MAP: Record<SubmitButtonState, React.CSSProperties> = {
  idle: { backgroundColor: "#4f46e5" },
  submitting: { backgroundColor: "#6366f1" },
  success: { backgroundColor: "#16a34a", borderColor: "#15803d" },
  error: { backgroundColor: "#dc2626", borderColor: "#b91c1c" },
  disabled: { backgroundColor: "#9ca3af", borderColor: "#9ca3af", cursor: "not-allowed", opacity: 0.9 },
};

// ── Pure helper functions (exported for unit testing) ─────────────────────────

/**
 * @notice Sanitizes a candidate label value before it is rendered.
 * @dev    Rejects non-strings, strips control characters, normalizes whitespace,
 *         and truncates to MAX_LABEL_LENGTH. Falls back to `fallback` on any failure.
 * @param candidate  Raw value from the labels prop (may be any type).
 * @param fallback   Known-safe default returned when candidate is unusable.
 * @return           A non-empty, bounded, printable string.
 * @security         Prevents blank UI states and limits layout-abuse via oversized labels.
 */
export function normalizeSubmitButtonLabel(candidate: unknown, fallback: string): string {
  if (typeof candidate !== "string") return fallback;

  const cleaned = candidate.replace(CONTROL_CHARACTER_REGEX, " ").replace(/\s+/g, " ").trim();
  if (!cleaned) return fallback;

  return cleaned.length <= MAX_LABEL_LENGTH
    ? cleaned
    : `${cleaned.slice(0, MAX_LABEL_LENGTH - 3)}...`;
}

/**
 * @notice Returns a safe, non-empty display label for the given state.
 * @dev    Delegates sanitization to normalizeSubmitButtonLabel; React renders the result
 *         as a text node, so no dangerouslySetInnerHTML path is exposed.
 * @param state   Current button state.
 * @param labels  Optional label overrides from the consumer.
 * @return        A printable, bounded label string.
 */
export function resolveSubmitButtonLabel(
  state: SubmitButtonState,
  labels?: SubmitButtonLabels,
): string {
  return normalizeSubmitButtonLabel(labels?.[state], DEFAULT_LABELS[state]);
}

/**
 * @notice Validates whether a transition from previousState to nextState is permitted.
 * @dev    Same-state transitions are always valid (idempotent updates).
 * @param previousState  The state the button is currently in.
 * @param nextState      The state the button is being asked to move to.
 * @return               true if the transition is in the allowlist or is a no-op.
 */
export function isValidSubmitButtonStateTransition(
  previousState: SubmitButtonState,
  nextState: SubmitButtonState,
): boolean {
  if (previousState === nextState) return true;
  return ALLOWED_STATE_TRANSITIONS[previousState].includes(nextState);
}

/**
 * @notice Resolves the effective state, blocking invalid transitions in strict mode.
 * @dev    When strictTransitions is true and the requested transition is invalid,
 *         the component stays in previousState rather than jumping to an illegal state.
 * @param state             Requested next state.
 * @param previousState     Current state (used for transition validation).
 * @param strictTransitions When false, all transitions are accepted without validation.
 * @return                  The safe effective state to render.
 */
export function resolveSafeSubmitButtonState(
  state: SubmitButtonState,
  previousState?: SubmitButtonState,
  strictTransitions = true,
): SubmitButtonState {
  if (!strictTransitions || !previousState) return state;
  return isValidSubmitButtonStateTransition(previousState, state) ? state : previousState;
}

/**
 * @notice Determines whether user interaction should be blocked.
 * @dev    Interaction is blocked when the button is submitting, externally disabled,
 *         or a local async click handler is still in-flight.
 * @param state               Effective button state.
 * @param disabled            External disabled prop.
 * @param isLocallySubmitting True while the component's own click handler awaits.
 * @return                    true if clicks should be suppressed.
 * @security                  Prevents duplicate blockchain transactions on rapid clicks.
 */
export function isSubmitButtonInteractionBlocked(
  state: SubmitButtonState,
  disabled = false,
  isLocallySubmitting = false,
): boolean {
  return Boolean(disabled) || state === "disabled" || state === "submitting" || isLocallySubmitting;
}

/**
 * @notice Determines whether the button should signal a busy/loading state.
 * @dev    Maps to the aria-busy attribute; true during active submission.
 * @param state               Effective button state.
 * @param isLocallySubmitting True while the component's own click handler awaits.
 * @return                    true when the button is actively processing.
 */
export function isSubmitButtonBusy(
  state: SubmitButtonState,
  isLocallySubmitting = false,
): boolean {
  return state === "submitting" || isLocallySubmitting;
}

// ── Component ─────────────────────────────────────────────────────────────────

/**
 * @title  ReactSubmitButton
 * @notice Reusable submit button with typed state machine, secure label rendering,
 *         double-submit prevention, and ARIA accessibility semantics.
 * @dev    - Resolves effective state via resolveSafeSubmitButtonState (memoized).
 *         - Tracks local in-flight execution to block re-entry before parent state updates.
 *         - Renders labels as React text nodes; no dangerouslySetInnerHTML is used.
 * @security Clicks are silently ignored while submitting or disabled (double-submit guard).
 */
const ReactSubmitButton = ({
  state,
  previousState,
  strictTransitions = true,
  labels,
  onClick,
  className,
  id,
  type = "button",
  disabled,
}: ReactSubmitButtonProps) => {
  /** @dev Tracks whether the local onClick handler is still awaiting resolution. */
  const [isLocallySubmitting, setIsLocallySubmitting] = useState(false);

  /**
   * @dev Memoized to avoid recomputing on every render when state/previousState are stable.
   *      Falls back to previousState when strictTransitions blocks an invalid jump.
   */
  const resolvedState = useMemo(
    () => resolveSafeSubmitButtonState(state, previousState, strictTransitions),
    [state, previousState, strictTransitions],
  );

  /** @dev Safe, bounded label for the current resolved state. */
  const label = resolveSubmitButtonLabel(resolvedState, labels);

  /** @dev True when clicks must be suppressed (submitting, disabled, or in-flight). */
  const computedDisabled = isSubmitButtonInteractionBlocked(resolvedState, disabled, isLocallySubmitting);

  /** @dev Drives aria-busy; true only during active submission. */
  const ariaBusy = isSubmitButtonBusy(resolvedState, isLocallySubmitting);

  /**
   * @notice Wraps the consumer's onClick to track local in-flight state.
   * @dev    setIsLocallySubmitting(false) runs in `finally` to guarantee cleanup
   *         even when onClick rejects, preventing a permanently blocked button.
   * @security Guard at the top prevents re-entry while already submitting.
   */
  const handleClick = async (event: React.MouseEvent<HTMLButtonElement>) => {
    if (computedDisabled || !onClick) return;

    setIsLocallySubmitting(true);
    try {
      await Promise.resolve(onClick(event));
    } finally {
      setIsLocallySubmitting(false);
    }
  };

  return (
    <button
      id={id}
      type={type}
      className={className}
      disabled={computedDisabled}
      aria-busy={ariaBusy}
      aria-live="polite"
      aria-label={label}
      onClick={computedDisabled ? undefined : handleClick}
      style={{
        ...BASE_STYLE,
        ...STATE_STYLE_MAP[resolvedState],
      }}
    >
      {label}
    </button>
  );
};

export default ReactSubmitButton;
