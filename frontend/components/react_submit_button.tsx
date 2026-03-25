/**
 * @title ReactSubmitButton
 * @notice Reusable submit button with a deterministic state machine for
 *         crowdfunding transaction flows (contribute, withdraw, refund).
 *
 * @dev State machine:
 *      idle ──click──► submitting ──resolve──► success ──(auto-reset)──► idle
 *                                └──reject──► error   ──(auto-reset)──► idle
 *
 * Security assumptions
 * --------------------
 * @security Labels are rendered as React text nodes — no dangerouslySetInnerHTML.
 * @security Clicks are silently ignored in submitting/success/disabled states
 *           to prevent duplicate blockchain transactions (double-spend).
 * @security All background colours are sourced from STATE_CONFIG constants —
 *           no dynamic CSS injection from user-supplied strings.
 * @security The auto-reset timer is cleared on unmount to prevent state
 *           updates on unmounted components (memory-leak protection).
 * @security Negative resetDelay values are clamped to 0.
 */

import React, { useEffect, useRef, useState } from "react";
import type { ButtonState, SubmitButtonProps } from "./react_submit_button_types";
import { STATE_CONFIG } from "./react_submit_button_types";

export type { ButtonState, SubmitButtonProps };

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_RESET_DELAY_MS = 2_500;

// ── Component ─────────────────────────────────────────────────────────────────

/**
 * @notice Accessible submit button with idle / submitting / success / error /
 *         disabled states.
 *
 * @param label       Text shown in idle and disabled states.
 * @param onClick     Async handler; rejection triggers error state.
 * @param disabled    When true, maps to the disabled state and blocks interaction.
 * @param resetDelay  Milliseconds before auto-reset from success/error. Default 2500.
 * @param type        HTML button type. Default "submit".
 * @param style       Additional inline styles merged onto the button element.
 * @param data-testid Optional test selector.
 */
const ReactSubmitButton: React.FC<SubmitButtonProps> = ({
  label,
  onClick,
  disabled = false,
  resetDelay = DEFAULT_RESET_DELAY_MS,
  type = "submit",
  style,
  "data-testid": testId,
}) => {
  const [internalState, setInternalState] = useState<ButtonState>("idle");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Map external disabled prop to the disabled state.
  const effectiveState: ButtonState = disabled ? "disabled" : internalState;

  const config = STATE_CONFIG[effectiveState];
  const resolvedLabel = effectiveState === "idle" || effectiveState === "disabled"
    ? label
    : config.label;

  const isInteractionBlocked =
    effectiveState === "submitting" ||
    effectiveState === "success" ||
    effectiveState === "disabled";

  // Clear any pending reset timer on unmount.
  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
    };
  }, []);

  const scheduleReset = (delay: number) => {
    const safeDelay = Math.max(0, delay);
    timerRef.current = setTimeout(() => {
      setInternalState("idle");
    }, safeDelay);
  };

  const handleClick = async () => {
    if (isInteractionBlocked || !onClick) return;

    setInternalState("submitting");
    try {
      await onClick();
      setInternalState("success");
      scheduleReset(resetDelay);
    } catch {
      setInternalState("error");
      scheduleReset(resetDelay);
    }
  };

  const buttonStyle: React.CSSProperties = {
    minHeight: "44px",
    minWidth: "120px",
    borderRadius: "8px",
    border: `1px solid ${config.backgroundColor}`,
    padding: "0.5rem 1rem",
    color: "#ffffff",
    fontWeight: 600,
    cursor: config.cursor,
    transition: "opacity 0.2s ease, background-color 0.2s ease",
    backgroundColor: config.backgroundColor,
    opacity: effectiveState === "disabled" ? 0.6 : 1,
    ...style,
  };

  return (
    <button
      type={type}
      disabled={isInteractionBlocked}
      aria-disabled={isInteractionBlocked}
      aria-busy={effectiveState === "submitting"}
      aria-live="polite"
      aria-label={config.ariaLabel || resolvedLabel}
      data-state={effectiveState}
      data-testid={testId}
      onClick={isInteractionBlocked ? undefined : handleClick}
      style={buttonStyle}
    >
      {resolvedLabel}
    </button>
  );
};

export default ReactSubmitButton;
