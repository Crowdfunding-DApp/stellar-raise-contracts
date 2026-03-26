/**
 * @title ReactSubmitButton — Test Suite
 * @notice Covers label normalization, state resolution, transition validation,
 *         interaction guards, busy semantics, and rendered component behaviour.
 * @dev Security assumptions are called out inline where relevant.
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
    expect(normalizeSubmitButtonLabel([], "Submit")).toBe("Submit");
  });

  it("returns fallback for empty string", () => {
    expect(normalizeSubmitButtonLabel("", "Submit")).toBe("Submit");
  });

  it("returns fallback for whitespace-only strings", () => {
    expect(normalizeSubmitButtonLabel("   ", "Submit")).toBe("Submit");
    expect(normalizeSubmitButtonLabel("\n\t", "Submit")).toBe("Submit");
  });

  it("strips control characters", () => {
    expect(normalizeSubmitButtonLabel("Pay\u0000Now", "Submit")).toBe("Pay Now");
    expect(normalizeSubmitButtonLabel("Pay\u0008\u001FNow", "Submit")).toBe("Pay Now");
    expect(normalizeSubmitButtonLabel("Pay\u007FNow", "Submit")).toBe("Pay Now");
  });

  it("normalizes internal whitespace", () => {
    expect(normalizeSubmitButtonLabel("Pay   Now", "Submit")).toBe("Pay Now");
    expect(normalizeSubmitButtonLabel("  Pay Now  ", "Submit")).toBe("Pay Now");
  });

  it("returns label unchanged when within the 80-char bound", () => {
    const label = "A".repeat(80);
    expect(normalizeSubmitButtonLabel(label, "Submit")).toBe(label);
  });

  it("truncates labels exceeding 80 characters with ellipsis", () => {
    const label = "A".repeat(100);
    const result = normalizeSubmitButtonLabel(label, "Submit");
    expect(result).toHaveLength(80);
    expect(result.endsWith("...")).toBe(true);
  });

  it("uses the fallback when the fallback itself is the only valid string", () => {
    expect(normalizeSubmitButtonLabel("", "Fallback")).toBe("Fallback");
  });
});

// ── resolveSubmitButtonLabel ──────────────────────────────────────────────────

describe("resolveSubmitButtonLabel", () => {
  it("returns correct defaults for every known state", () => {
    const states: SubmitButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    const labels = states.map((s) => resolveSubmitButtonLabel(s));
    expect(labels).toEqual(["Submit", "Submitting...", "Submitted", "Try Again", "Submit Disabled"]);
  });

  it("uses a valid custom label", () => {
    const custom: SubmitButtonLabels = { idle: "Fund Campaign" };
    expect(resolveSubmitButtonLabel("idle", custom)).toBe("Fund Campaign");
  });

  it("falls back to default when custom label is empty", () => {
    expect(resolveSubmitButtonLabel("idle", { idle: "" })).toBe("Submit");
  });

  it("falls back to default when custom label is whitespace", () => {
    expect(resolveSubmitButtonLabel("submitting", { submitting: "   " })).toBe("Submitting...");
  });

  it("trims leading and trailing whitespace from custom labels", () => {
    expect(resolveSubmitButtonLabel("success", { success: "  Done  " })).toBe("Done");
  });

  it("truncates an overly long custom label", () => {
    const long = "A".repeat(100);
    const result = resolveSubmitButtonLabel("error", { error: long });
    expect(result).toHaveLength(80);
    expect(result.endsWith("...")).toBe(true);
  });

  it("keeps markup-like text as inert string content (XSS note)", () => {
    // Security: React renders this as a text node, not executable HTML.
    const hostile = "<img src=x onerror=alert(1) />";
    expect(resolveSubmitButtonLabel("error", { error: hostile })).toBe(hostile);
  });

  it("resolves independently for each state when all overrides are provided", () => {
    const custom: SubmitButtonLabels = {
      idle: "Send",
      submitting: "Sending",
      success: "Sent",
      error: "Retry",
      disabled: "Locked",
    };
    expect(resolveSubmitButtonLabel("idle", custom)).toBe("Send");
    expect(resolveSubmitButtonLabel("submitting", custom)).toBe("Sending");
    expect(resolveSubmitButtonLabel("success", custom)).toBe("Sent");
    expect(resolveSubmitButtonLabel("error", custom)).toBe("Retry");
    expect(resolveSubmitButtonLabel("disabled", custom)).toBe("Locked");
  });
});

// ── isValidSubmitButtonStateTransition ───────────────────────────────────────

describe("isValidSubmitButtonStateTransition", () => {
  it("allows expected forward transitions", () => {
    expect(isValidSubmitButtonStateTransition("idle", "submitting")).toBe(true);
    expect(isValidSubmitButtonStateTransition("submitting", "success")).toBe(true);
    expect(isValidSubmitButtonStateTransition("submitting", "error")).toBe(true);
    expect(isValidSubmitButtonStateTransition("error", "submitting")).toBe(true);
    expect(isValidSubmitButtonStateTransition("error", "idle")).toBe(true);
    expect(isValidSubmitButtonStateTransition("success", "idle")).toBe(true);
    expect(isValidSubmitButtonStateTransition("disabled", "idle")).toBe(true);
  });

  it("allows any state to transition to disabled", () => {
    const states: SubmitButtonState[] = ["idle", "submitting", "success", "error"];
    states.forEach((s) => {
      expect(isValidSubmitButtonStateTransition(s, "disabled")).toBe(true);
    });
  });

  it("allows same-state transitions (idempotent updates)", () => {
    const states: SubmitButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    states.forEach((s) => {
      expect(isValidSubmitButtonStateTransition(s, s)).toBe(true);
    });
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
  it("returns requested state when transition is valid in strict mode", () => {
    expect(resolveSafeSubmitButtonState("submitting", "idle", true)).toBe("submitting");
    expect(resolveSafeSubmitButtonState("success", "submitting", true)).toBe("success");
  });

  it("falls back to previousState for invalid transitions in strict mode", () => {
    expect(resolveSafeSubmitButtonState("success", "idle", true)).toBe("idle");
    expect(resolveSafeSubmitButtonState("error", "success", true)).toBe("success");
  });

  it("accepts any requested state when strict mode is disabled", () => {
    expect(resolveSafeSubmitButtonState("success", "idle", false)).toBe("success");
    expect(resolveSafeSubmitButtonState("error", "disabled", false)).toBe("error");
  });

  it("accepts requested state when previousState is undefined", () => {
    expect(resolveSafeSubmitButtonState("error", undefined, true)).toBe("error");
    expect(resolveSafeSubmitButtonState("success", undefined, true)).toBe("success");
  });

  it("defaults strictTransitions to true", () => {
    // idle → success is invalid; should fall back to idle
    expect(resolveSafeSubmitButtonState("success", "idle")).toBe("idle");
  });
});

// ── isSubmitButtonInteractionBlocked ─────────────────────────────────────────

describe("isSubmitButtonInteractionBlocked", () => {
  it("blocks interaction for submitting state", () => {
    expect(isSubmitButtonInteractionBlocked("submitting")).toBe(true);
  });

  it("blocks interaction for disabled state", () => {
    expect(isSubmitButtonInteractionBlocked("disabled")).toBe(true);
  });

  it("blocks interaction when explicit disabled flag is true", () => {
    expect(isSubmitButtonInteractionBlocked("idle", true)).toBe(true);
    expect(isSubmitButtonInteractionBlocked("success", true)).toBe(true);
    expect(isSubmitButtonInteractionBlocked("error", true)).toBe(true);
  });

  it("blocks interaction when isLocallySubmitting is true", () => {
    expect(isSubmitButtonInteractionBlocked("idle", false, true)).toBe(true);
    expect(isSubmitButtonInteractionBlocked("error", false, true)).toBe(true);
  });

  it("allows interaction for idle, success, and error when all flags are clear", () => {
    expect(isSubmitButtonInteractionBlocked("idle", false, false)).toBe(false);
    expect(isSubmitButtonInteractionBlocked("success", false, false)).toBe(false);
    expect(isSubmitButtonInteractionBlocked("error", false, false)).toBe(false);
  });
});

// ── isSubmitButtonBusy ────────────────────────────────────────────────────────

describe("isSubmitButtonBusy", () => {
  it("is true when state is submitting", () => {
    expect(isSubmitButtonBusy("submitting")).toBe(true);
  });

  it("is true when isLocallySubmitting is true regardless of state", () => {
    expect(isSubmitButtonBusy("idle", true)).toBe(true);
    expect(isSubmitButtonBusy("error", true)).toBe(true);
  });

  it("is false for non-submitting states with no local flag", () => {
    const states: SubmitButtonState[] = ["idle", "success", "error", "disabled"];
    states.forEach((s) => {
      expect(isSubmitButtonBusy(s, false)).toBe(false);
    });
  });
});

// ── ReactSubmitButton component ───────────────────────────────────────────────

describe("ReactSubmitButton", () => {
  it("renders with the default idle label", () => {
    render(<ReactSubmitButton state="idle" />);
    expect(screen.getByRole("button")).toHaveTextContent("Submit");
  });

  it("renders the correct label for each state", () => {
    const cases: [SubmitButtonState, string][] = [
      ["idle", "Submit"],
      ["submitting", "Submitting..."],
      ["success", "Submitted"],
      ["error", "Try Again"],
      ["disabled", "Submit Disabled"],
    ];
    cases.forEach(([state, expected]) => {
      const { unmount } = render(<ReactSubmitButton state={state} />);
      expect(screen.getByRole("button")).toHaveTextContent(expected);
      unmount();
    });
  });

  it("renders a custom label override", () => {
    render(<ReactSubmitButton state="idle" labels={{ idle: "Fund Campaign" }} />);
    expect(screen.getByRole("button")).toHaveTextContent("Fund Campaign");
  });

  it("is disabled in submitting state", () => {
    render(<ReactSubmitButton state="submitting" />);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("is disabled in disabled state", () => {
    render(<ReactSubmitButton state="disabled" />);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("is disabled when explicit disabled prop is true", () => {
    render(<ReactSubmitButton state="idle" disabled />);
    expect(screen.getByRole("button")).toBeDisabled();
  });

  it("is not disabled in idle, success, or error states", () => {
    const states: SubmitButtonState[] = ["idle", "success", "error"];
    states.forEach((state) => {
      const { unmount } = render(<ReactSubmitButton state={state} />);
      expect(screen.getByRole("button")).not.toBeDisabled();
      unmount();
    });
  });

  it("sets aria-busy true when submitting", () => {
    render(<ReactSubmitButton state="submitting" />);
    expect(screen.getByRole("button")).toHaveAttribute("aria-busy", "true");
  });

  it("sets aria-busy false when not submitting", () => {
    render(<ReactSubmitButton state="idle" />);
    expect(screen.getByRole("button")).toHaveAttribute("aria-busy", "false");
  });

  it("calls onClick when clicked in idle state", async () => {
    const handler = jest.fn().mockResolvedValue(undefined);
    render(<ReactSubmitButton state="idle" onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    await waitFor(() => expect(handler).toHaveBeenCalledTimes(1));
  });

  it("does not call onClick when in submitting state", () => {
    const handler = jest.fn();
    render(<ReactSubmitButton state="submitting" onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });

  it("does not call onClick when in disabled state", () => {
    const handler = jest.fn();
    render(<ReactSubmitButton state="disabled" onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });

  it("does not call onClick when explicit disabled prop is true", () => {
    const handler = jest.fn();
    render(<ReactSubmitButton state="idle" disabled onClick={handler} />);
    fireEvent.click(screen.getByRole("button"));
    expect(handler).not.toHaveBeenCalled();
  });

  it("applies the id prop", () => {
    render(<ReactSubmitButton state="idle" id="submit-btn" />);
    expect(screen.getByRole("button")).toHaveAttribute("id", "submit-btn");
  });

  it("applies the className prop", () => {
    render(<ReactSubmitButton state="idle" className="my-btn" />);
    expect(screen.getByRole("button")).toHaveClass("my-btn");
  });

  it("defaults type to button", () => {
    render(<ReactSubmitButton state="idle" />);
    expect(screen.getByRole("button")).toHaveAttribute("type", "button");
  });

  it("respects an explicit type prop", () => {
    render(<ReactSubmitButton state="idle" type="submit" />);
    expect(screen.getByRole("button")).toHaveAttribute("type", "submit");
  });

  it("falls back to previousState when strict transition is invalid", () => {
    // idle → success is not a valid transition; resolvedState should stay idle
    render(
      <ReactSubmitButton state="success" previousState="idle" strictTransitions={true} />,
    );
    expect(screen.getByRole("button")).toHaveTextContent("Submit");
  });

  it("accepts an invalid transition when strictTransitions is false", () => {
    render(
      <ReactSubmitButton state="success" previousState="idle" strictTransitions={false} />,
    );
    expect(screen.getByRole("button")).toHaveTextContent("Submitted");
  });
});
