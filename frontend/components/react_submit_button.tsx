import React, { useMemo, useState } from "react";

/**
 * @title ReactSubmitButton
 * @notice Reusable submit button with typed state machine, secure label handling,
 *         interaction guards, and full accessibility semantics.
 * @dev Clicks are blocked while submitting or disabled to prevent duplicate submissions.
 *      Labels are sanitized — control characters stripped, length bounded to 80 chars.
 *      All colours are hardcoded constants; no user input is interpolated into CSS.
 */

export type SubmitButtonState = "idle" | "submitting" | "success" | "error" | "disabled";

/**
 * @notice Optional label overrides for each button state.
 */
export interface SubmitButtonLabels {
  idle?: string;
  submitting?: string;
  success?: string;
  error?: string;
  disabled?: string;
}

/**
 * @notice Props accepted by ReactSubmitButton.
 * @param state         Current state driving visual and interaction behaviour.
 * @param previousState Last known state; used by strict transition guard.
 * @param strictTransitions When true, invalid transitions fall back to previousState.
 * @param labels        Optional label overrides per state.
 * @param onClick       Handler called on click; may be async.
 * @param className     Optional CSS class.
 * @param id            Optional element id.
 * @param type          HTML button type. Defaults to "button".
 * @param disabled      External disabled flag.
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

const DEFAULT_LABELS: Required<SubmitButtonLabels> = {
  idle: "Submit",
  submitting: "Submitting...",
  success: "Submitted",
  error: "Try Again",
  disabled: "Submit Disabled",
};

const MAX_LABEL_LENGTH = 80;
const CONTROL_CHARACTER_REGEX = /[\u0000-\u001F\u007F]/g;

const ALLOWED_STATE_TRANSITIONS: Record<SubmitButtonState, SubmitButtonState[]> = {
  idle: ["submitting", "disabled"],
  submitting: ["success", "error", "disabled"],
  success: ["idle", "disabled"],
  error: ["idle", "submitting", "disabled"],
  disabled: ["idle"],
};

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

const STATE_STYLE_MAP: Record<SubmitButtonState, React.CSSProperties> = {
  idle: { backgroundColor: "#4f46e5" },
  submitting: { backgroundColor: "#6366f1" },
  success: { backgroundColor: "#16a34a", borderColor: "#15803d" },
  error: { backgroundColor: "#dc2626", borderColor: "#b91c1c" },
  disabled: { backgroundColor: "#9ca3af", borderColor: "#9ca3af", cursor: "not-allowed", opacity: 0.9 },
};

/**
 * @notice Strips control characters, normalizes whitespace, and bounds label length.
 * @dev React escapes text nodes by default; this function only normalizes for UX safety.
 * @param candidate Raw label value (may be any type).
 * @param fallback  Returned when candidate is unusable.
 */
export function normalizeSubmitButtonLabel(candidate: unknown, fallback: string): string {
  if (typeof candidate !== "string") return fallback;
  const cleaned = candidate.replace(CONTROL_CHARACTER_REGEX, " ").replace(/\s+/g, " ").trim();
  if (!cleaned) return fallback;
  if (cleaned.length <= MAX_LABEL_LENGTH) return cleaned;
  return `${cleaned.slice(0, MAX_LABEL_LENGTH - 3)}...`;
}

/**
 * @notice Returns a safe, non-empty label for the given state.
 * @param state  Current button state.
 * @param labels Optional label overrides.
 */
export function resolveSubmitButtonLabel(
  state: SubmitButtonState,
  labels?: SubmitButtonLabels,
): string {
  return normalizeSubmitButtonLabel(labels?.[state], DEFAULT_LABELS[state]);
}

/**
 * @notice Returns true when the transition from previousState to nextState is permitted.
 * @dev Same-state transitions are always allowed (idempotent updates).
 */
export function isValidSubmitButtonStateTransition(
  previousState: SubmitButtonState,
  nextState: SubmitButtonState,
): boolean {
  if (previousState === nextState) return true;
  return ALLOWED_STATE_TRANSITIONS[previousState].includes(nextState);
}

/**
 * @notice Resolves the final state, blocking invalid transitions in strict mode.
 * @param state            Requested next state.
 * @param previousState    Last known state.
 * @param strictTransitions When true, invalid transitions return previousState.
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
 * @notice Returns true when the button should not respond to clicks.
 * @security Prevents duplicate submissions during in-flight async handlers.
 */
export function isSubmitButtonInteractionBlocked(
  state: SubmitButtonState,
  disabled = false,
  isLocallySubmitting = false,
): boolean {
  return Boolean(disabled) || state === "disabled" || state === "submitting" || isLocallySubmitting;
}

/**
 * @notice Returns true when aria-busy should be set.
 */
export function isSubmitButtonBusy(
  state: SubmitButtonState,
  isLocallySubmitting = false,
): boolean {
  return state === "submitting" || isLocallySubmitting;
}

/**
 * @title ReactSubmitButton
 * @notice Accessible submit button driven by an explicit state machine.
 * @dev onClick is wrapped to set a local in-flight flag, blocking re-entry
 *      until the handler resolves or rejects.
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
  const [isLocallySubmitting, setIsLocallySubmitting] = useState(false);

  const resolvedState = useMemo(
    () => resolveSafeSubmitButtonState(state, previousState, strictTransitions),
    [state, previousState, strictTransitions],
  );

  const label = resolveSubmitButtonLabel(resolvedState, labels);
  const computedDisabled = isSubmitButtonInteractionBlocked(resolvedState, disabled, isLocallySubmitting);
  const ariaBusy = isSubmitButtonBusy(resolvedState, isLocallySubmitting);

  const handleClick = async (event: React.MouseEvent<HTMLButtonElement>) => {
    if (computedDisabled || !onClick) return;
    setIsLocallySubmitting(true);
    try {
      await Promise.resolve(onClick(event));
    } catch {
      // Rejection is intentionally swallowed here; callers surface errors via
      // the `state` prop (e.g. transitioning to "error"). Re-throwing would
      // produce an unhandled rejection in the browser.
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
      style={{ ...BASE_STYLE, ...STATE_STYLE_MAP[resolvedState] }}
    >
      {label}
    </button>
  );
};

export default ReactSubmitButton;
