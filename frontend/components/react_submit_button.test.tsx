/**
 * @title  React Submit Button — Test Suite
 * @notice Validates label normalization, state transitions, interaction guards,
 *         busy-state semantics, security assumptions, and component rendering.
 * @dev    Pure-function tests require no DOM; component tests use @testing-library/react.
 *         Security notes are inline where hostile inputs are exercised.
 */
import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import ReactSubmitButton, {
  isSubmitButtonBusy,
  isSubmitButtonInteractionBlocked,
  isValidSubmitButtonStateTransition,
  normalizeSubmitButtonLabel,
  resolveSafeSubmitButtonState,
  resolveSubmitButtonLabel,
  type SubmitButtonLabels,
  type SubmitButtonState,
} from "./react_submit_button";

// ── normalizeSubmitButtonLabel ────────────────────────────────────────────────

describe("normalizeSubmitButtonLabel", () => {
  it("returns fallback for non-string values", () => {
    expect(normalizeSubmitButtonLabel(undefined, "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel(null, "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel(404, "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel({}, "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel(true, "Submit")).toBe("Submit");
  });

  it("returns fallback for empty or whitespace-only strings", () => {
    expect(normalizeSubmitButtonLabel("", "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel("   ", "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel("\n\t", "Submit")).toBe("Submit");
  });

  it("strips control characters and normalizes internal whitespace", () => {
    // Security: control characters from untrusted sources are removed before rendering.
    expect(normalizeSubmitButtonLabel("Pay\u0000\u0008\nNow", "Submit")).toBe("Pay Now");
    expect(normalizeSubmitButtonLabel("A\u001FB", "Submit")).toBe("A B");
    expect(normalizeSubmitButtonLabel("A\u007FB", "Submit")).toBe("A B");
  });

  it("trims leading and trailing whitespace", () => {
    expect(normalizeSubmitButtonLabel("  Submit  ", "Fallback")).toBe("Submit");
  });

  it("truncates labels that exceed 80 characters", () => {
    const long = "A".repeat(200);
    const result = normalizeSubmitButtonLabel(long, "Submit");
    expect(result).toHaveLength(80);
    expect(result.endsWith("...")).toBe(true);
  });

  it("returns labels at exactly 80 characters without truncation", () => {
    const exact = "B".repeat(80);
    expect(normalizeSubmitButtonLabel(exact, "Submit")).toBe(exact);
  });

  it("returns labels shorter than 80 characters unchanged", () => {
    expect(normalizeSubmitButtonLabel("Short label", "Submit")).toBe("Short label");
  });
});

// ── resolveSubmitButtonLabel ──────────────────────────────────────────────────

describe("resolveSubmitButtonLabel", () => {
  it("returns the correct default label for every known state", () => {
    const states: SubmitButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    const labels = states.map((s) => resolveSubmitButtonLabel(s));
    expect(labels).toEqual(["Submit", "Submitting...", "Submitted", "Try Again", "Submit Disabled"]);
  });

  it("uses a valid custom label override", () => {
    const custom: SubmitButtonLabels = { idle: "Fund Campaign" };
    expect(resolveSubmitButtonLabel("idle", custom)).toBe("Fund Campaign");
  });

  it("falls back to default when the override is empty", () => {
    expect(resolveSubmitButtonLabel("idle", { idle: "" })).toBe("Submit");
  });

  it("falls back to default when the override is whitespace-only", () => {
    expect(resolveSubmitButtonLabel("submitting", { submitting: "   " })).toBe("Submitting...");
  });

  it("truncates an oversized custom label", () => {
    const long = "X".repeat(100);
    const result = resolveSubmitButtonLabel("success", { success: long });
    expect(result).toHaveLength(80);
    expect(result.endsWith("...")).toBe(true);
  });

  it("preserves hostile markup-like text as an inert string", () => {
    // Security: React renders this as a text node, not executable HTML.
    const hostile = "<img src=x onerror=alert(1) />";
    expect(resolveSubmitButtonLabel("error", { error: hostile })).toBe(hostile);
  });
});

// ── isValidSubmitButtonStateTransition ───────────────────────────────────────

describe("isValidSubmitButtonStateTransition", () => {
  it("allows all expected forward transitions", () => {
    expect(isValidSubmitButtonStateTransition("idle", "submitting")).toBe(true);
    expect(isValidSubmitButtonStateTransition("idle", "disabled")).toBe(true);
    expect(isValidSubmitButtonStateTransition("submitting", "success")).toBe(true);
    expect(isValidSubmitButtonStateTransition("submitting", "error")).toBe(true);
    expect(isValidSubmitButtonStateTransition("submitting", "disabled")).toBe(true);
    expect(isValidSubmitButtonStateTransition("success", "idle")).toBe(true);
    expect(isValidSubmitButtonStateTransition("success", "disabled")).toBe(true);
    expect(isValidSubmitButtonStateTransition("error", "idle")).toBe(true);
    expect(isValidSubmitButtonStateTransition("error", "submitting")).toBe(true);
    expect(isValidSubmitButtonStateTransition("error", "disabled")).toBe(true);
    expect(isValidSubmitButtonStateTransition("disabled", "idle")).toBe(true);
  });

  it("allows same-state transitions (idempotent updates)", () => {
    const states: SubmitButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    states.forEach((s) => expect(isValidSubmitButtonStateTransition(s, s)).toBe(true));
  });

  it("blocks invalid transitions", () => {
    expect(isValidSubmitButtonStateTransition("idle", "success")).toBe(false);
    expect(isValidSubmitButtonStateTransition("idle", "error")).toBe(false);
    expect(isValidSubmitButtonStateTransition("success", "error")).toBe(false);
    expect(isValidSubmitButtonStateTransition("success", "submitting")).toBe(false);
    expect(isValidSubmitButtonStateTransition("disabled", "submitting")).toBe(false);
    expect(isValidSubmitButtonStateTransition("disabled", "success")).toBe(false);
    expect(isValidSubmitButtonStateTransition("disabled", "error")).toBe(false);
  });
});

// ── resolveSafeSubmitButtonState ─────────────────────────────────────────────

describe("resolveSafeSubmitButtonState", () => {
  it("returns the requested state when the transition is valid", () => {
    expect(resolveSafeSubmitButtonState("submitting", "idle", true)).toBe("submitting");
    expect(resolveSafeSubmitButtonState("success", "submitting", true)).toBe("success");
  });

  it("falls back to previousState when the transition is invalid in strict mode", () => {
    // Security: prevents race-condition state jumps (e.g. idle → success).
    expect(resolveSafeSubmitButtonState("success", "idle", true)).toBe("idle");
    expect(resolveSafeSubmitButtonState("error", "success", true)).toBe("success");
  });

  it("accepts any state when strict mode is disabled", () => {
    expect(resolveSafeSubmitButtonState("success", "idle", false)).toBe("success");
    expect(resolveSafeSubmitButtonState("error", "success", false)).toBe("error");
  });

  it("accepts any state when previousState is not provided", () => {
    expect(resolveSafeSubmitButtonState("error", undefined, true)).toBe("error");
    expect(resolveSafeSubmitButtonState("success", undefined, true)).toBe("success");
  });

  it("defaults strictTransitions to true", () => {
    // idle → success is invalid; should fall back to idle.
    expect(resolveSafeSubmitButtonState("success", "idle")).toBe("idle");
  });
});

// ── isSubmitButtonInteractionBlocked ─────────────────────────────────────────

describe("isSubmitButtonInteractionBlocked", () => {
  it("blocks interaction for the submitting state", () => {
    expect(isSubmitButtonInteractionBlocked("submitting")).toBe(true);
  });

  it("blocks interaction for the disabled state", () => {
    expect(isSubmitButtonInteractionBlocked("disabled")).toBe(true);
  });

  it("blocks interaction when the explicit disabled flag is true", () => {
    expect(isSubmitButtonInteractionBlocked("idle", true)).toBe(true);
    expect(isSubmitButtonInteractionBlocked("success", true)).toBe(true);
  });

  it("blocks interaction when a local async handler is in-flight", () => {
    // Security: prevents duplicate submissions before parent state updates.
    expect(isSubmitButtonInteractionBlocked("idle", false, true)).toBe(true);
  });

  it("allows interaction for active states when all flags are clear", () => {
    expect(isSubmitButtonInteractionBlocked("idle", false, false)).toBe(false);
    expect(isSubmitButtonInteractionBlocked("error", false, false)).toBe(false);
    expect(isSubmitButtonInteractionBlocked("success", false, false)).toBe(false);
  });
});

// ── isSubmitButtonBusy ────────────────────────────────────────────────────────

describe("isSubmitButtonBusy", () => {
  it("is true while in the submitting state", () => {
    expect(isSubmitButtonBusy("submitting", false)).toBe(true);
  });

  it("is true while a local async handler is in-flight", () => {
    expect(isSubmitButtonBusy("idle", true)).toBe(true);
  });

  it("is false for all non-submitting states when no local handler is in-flight", () => {
    const states: SubmitButtonState[] = ["idle", "success", "error", "disabled"];
    states.forEach((s) => expect(isSubmitButtonBusy(s, false)).toBe(false));
  });

  it("defaults isLocallySubmitting to false", () => {
    expect(isSubmitButtonBusy("idle")).toBe(false);
    expect(isSubmitButtonBusy("submitting")).toBe(true);
  });
});

// ── ReactSubmitButton component ───────────────────────────────────────────────

describe("ReactSubmitButton", () => {
  it("renders the default idle label", () => {
    render(<ReactSubmitButton state="idle" />);
    expect(screen.getByRole("button")).toHaveTextContent("Submit");
  });

  it("renders a custom label override", () => {
    render(<ReactSubmitButton state="idle" labels={{ idle: "Fund Campaign" }} />);
    expect(screen.getByRole("button")).toHaveTextContent("Fund Campaign");
  });

  it("is disabled and aria-busy while submitting", () => {
    render(<ReactSubmitButton state="submitting" />);
    const btn = screen.getByRole("button");
    expect(btn).toBeDisabled();
    expect(btn).toHaveAttribute("aria-busy", "true");
  });

  it("is disabled for the disabled state", () => {
    render(<ReactSubmitButton state="disabled" />);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("is disabled when the explicit disabled prop is true", () => {
    render(<ReactSubmitButton state="idle" disabled />);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("calls onClick when idle and not disabled", async () => {
    const handler = jest.fn().mockResolvedValue(undefined);
    render(<ReactSubmitButton state="idle" onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    await waitFor(() => expect(handler).toHaveBeenCalledTimes(1));
  });

  it("does not call onClick when disabled", () => {
    const handler = jest.fn();
    render(<ReactSubmitButton state="disabled" onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });

  it("applies the id and className props", () => {
    render(<ReactSubmitButton state="idle" id="submit-btn" className="custom" />);
    const btn = screen.getByRole("button");
    expect(btn).toHaveAttribute("id", "submit-btn");
    expect(btn).toHaveClass("custom");
  });

  it("falls back to previousState on an invalid strict transition", () => {
    // idle → success is invalid; button should show idle label.
    render(<ReactSubmitButton state="success" previousState="idle" strictTransitions />);
    expect(screen.getByRole("button")).toHaveTextContent("Submit");
  });

  it("renders success state correctly", () => {
    render(<ReactSubmitButton state="success" previousState="submitting" />);
    expect(screen.getByRole("button")).toHaveTextContent("Submitted");
  });

  it("renders error state correctly", () => {
    render(<ReactSubmitButton state="error" previousState="submitting" />);
    expect(screen.getByRole("button")).toHaveTextContent("Try Again");
  });
});
