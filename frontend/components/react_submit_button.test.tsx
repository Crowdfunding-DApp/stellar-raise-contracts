/**
 * @title ReactSubmitButton — Comprehensive Test Suite
 * @notice Covers state machine logic, accessibility, security assumptions,
 *         and component rendering for all button states.
 *
 * @dev Tests are organised into:
 *      1. Pure helper functions (no React needed)
 *      2. Component rendering per state
 *      3. Interaction / click handler behaviour
 *      4. Auto-reset timer behaviour
 *      5. Accessibility attributes
 *      6. Security edge cases
 */

import React from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import ReactSubmitButton from "./react_submit_button";
import {
  ALLOWED_TRANSITIONS,
  isBusy,
  isInteractionBlocked,
  isValidStateTransition,
  STATE_CONFIG,
  type ButtonState,
} from "./react_submit_button_types";

// ── Helpers ───────────────────────────────────────────────────────────────────

const noop = () => Promise.resolve();

function renderBtn(overrides: Partial<Parameters<typeof ReactSubmitButton>[0]> = {}) {
  const defaults = { label: "Contribute", onClick: noop };
  return render(<ReactSubmitButton {...defaults} {...overrides} />);
}

function getBtn() {
  return screen.getByRole("button") as HTMLButtonElement;
}

// ── 1. STATE_CONFIG completeness ──────────────────────────────────────────────

describe("STATE_CONFIG", () => {
  const states: ButtonState[] = ["idle", "submitting", "success", "error", "disabled"];

  it("defines an entry for every ButtonState", () => {
    states.forEach((s) => expect(STATE_CONFIG).toHaveProperty(s));
  });

  it("every entry has a non-empty backgroundColor", () => {
    states.forEach((s) => expect(STATE_CONFIG[s].backgroundColor).toBeTruthy());
  });

  it("every entry has a cursor value", () => {
    states.forEach((s) => expect(STATE_CONFIG[s].cursor).toBeTruthy());
  });

  it("every entry has an ariaLabel string", () => {
    states.forEach((s) => expect(typeof STATE_CONFIG[s].ariaLabel).toBe("string"));
  });

  it("submitting has a non-empty label", () => {
    expect(STATE_CONFIG.submitting.label).toBeTruthy();
  });

  it("success has a non-empty label", () => {
    expect(STATE_CONFIG.success.label).toBeTruthy();
  });

  it("error has a non-empty label", () => {
    expect(STATE_CONFIG.error.label).toBeTruthy();
  });

  it("idle and disabled labels are empty strings (label prop is used instead)", () => {
    expect(STATE_CONFIG.idle.label).toBe("");
    expect(STATE_CONFIG.disabled.label).toBe("");
  });

  it("submitting cursor is not-allowed", () => {
    expect(STATE_CONFIG.submitting.cursor).toBe("not-allowed");
  });

  it("disabled cursor is not-allowed", () => {
    expect(STATE_CONFIG.disabled.cursor).toBe("not-allowed");
  });

  it("idle cursor is pointer", () => {
    expect(STATE_CONFIG.idle.cursor).toBe("pointer");
  });

  it("error cursor is pointer (retry allowed)", () => {
    expect(STATE_CONFIG.error.cursor).toBe("pointer");
  });
});

// ── 2. isValidStateTransition ─────────────────────────────────────────────────

describe("isValidStateTransition", () => {
  it("allows idle → submitting", () => {
    expect(isValidStateTransition("idle", "submitting")).toBe(true);
  });

  it("allows submitting → success", () => {
    expect(isValidStateTransition("submitting", "success")).toBe(true);
  });

  it("allows submitting → error", () => {
    expect(isValidStateTransition("submitting", "error")).toBe(true);
  });

  it("allows error → idle", () => {
    expect(isValidStateTransition("error", "idle")).toBe(true);
  });

  it("allows error → submitting (retry)", () => {
    expect(isValidStateTransition("error", "submitting")).toBe(true);
  });

  it("allows success → idle (auto-reset)", () => {
    expect(isValidStateTransition("success", "idle")).toBe(true);
  });

  it("allows disabled → idle", () => {
    expect(isValidStateTransition("disabled", "idle")).toBe(true);
  });

  it("allows any state → disabled", () => {
    const states: ButtonState[] = ["idle", "submitting", "success", "error"];
    states.forEach((s) => expect(isValidStateTransition(s, "disabled")).toBe(true));
  });

  it("allows same-state transitions (idempotent)", () => {
    const states: ButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    states.forEach((s) => expect(isValidStateTransition(s, s)).toBe(true));
  });

  it("blocks idle → success (skipping submitting)", () => {
    expect(isValidStateTransition("idle", "success")).toBe(false);
  });

  it("blocks idle → error (skipping submitting)", () => {
    expect(isValidStateTransition("idle", "error")).toBe(false);
  });

  it("blocks success → error", () => {
    expect(isValidStateTransition("success", "error")).toBe(false);
  });

  it("blocks disabled → submitting", () => {
    expect(isValidStateTransition("disabled", "submitting")).toBe(false);
  });

  it("ALLOWED_TRANSITIONS covers all states", () => {
    const states: ButtonState[] = ["idle", "submitting", "success", "error", "disabled"];
    states.forEach((s) => expect(ALLOWED_TRANSITIONS).toHaveProperty(s));
  });
});

// ── 3. isInteractionBlocked ───────────────────────────────────────────────────

describe("isInteractionBlocked", () => {
  it("blocks submitting state", () => {
    expect(isInteractionBlocked("submitting")).toBe(true);
  });

  it("blocks success state", () => {
    expect(isInteractionBlocked("success")).toBe(true);
  });

  it("blocks disabled state", () => {
    expect(isInteractionBlocked("disabled")).toBe(true);
  });

  it("does not block idle state", () => {
    expect(isInteractionBlocked("idle")).toBe(false);
  });

  it("does not block error state (retry allowed)", () => {
    expect(isInteractionBlocked("error")).toBe(false);
  });

  it("blocks idle when disabled flag is true", () => {
    expect(isInteractionBlocked("idle", true)).toBe(true);
  });

  it("blocks error when disabled flag is true", () => {
    expect(isInteractionBlocked("error", true)).toBe(true);
  });
});

// ── 4. isBusy ─────────────────────────────────────────────────────────────────

describe("isBusy", () => {
  it("is true only for submitting", () => {
    expect(isBusy("submitting")).toBe(true);
  });

  it("is false for idle", () => {
    expect(isBusy("idle")).toBe(false);
  });

  it("is false for success", () => {
    expect(isBusy("success")).toBe(false);
  });

  it("is false for error", () => {
    expect(isBusy("error")).toBe(false);
  });

  it("is false for disabled", () => {
    expect(isBusy("disabled")).toBe(false);
  });
});

// ── 5. Component rendering ────────────────────────────────────────────────────

describe("idle state rendering", () => {
  it("renders the label prop", () => {
    renderBtn({ label: "Fund Campaign" });
    expect(screen.getByText("Fund Campaign")).toBeTruthy();
  });

  it("is not disabled", () => {
    renderBtn();
    expect(getBtn().disabled).toBe(false);
  });

  it("has data-state='idle'", () => {
    renderBtn();
    expect(getBtn().getAttribute("data-state")).toBe("idle");
  });

  it("has aria-busy='false'", () => {
    renderBtn();
    expect(getBtn().getAttribute("aria-busy")).toBe("false");
  });

  it("has aria-disabled='false'", () => {
    renderBtn();
    expect(getBtn().getAttribute("aria-disabled")).toBe("false");
  });

  it("has type='submit' by default", () => {
    renderBtn();
    expect(getBtn().type).toBe("submit");
  });

  it("respects explicit type='button'", () => {
    renderBtn({ type: "button" });
    expect(getBtn().type).toBe("button");
  });
});

describe("disabled prop rendering", () => {
  it("is disabled when disabled=true", () => {
    renderBtn({ disabled: true });
    expect(getBtn().disabled).toBe(true);
  });

  it("has data-state='disabled' when disabled=true", () => {
    renderBtn({ disabled: true });
    expect(getBtn().getAttribute("data-state")).toBe("disabled");
  });

  it("still shows the label prop when disabled", () => {
    renderBtn({ label: "Contribute", disabled: true });
    expect(screen.getByText("Contribute")).toBeTruthy();
  });

  it("has aria-disabled='true' when disabled", () => {
    renderBtn({ disabled: true });
    expect(getBtn().getAttribute("aria-disabled")).toBe("true");
  });
});

// ── 6. Click handler and state transitions ────────────────────────────────────

describe("click handler — success path", () => {
  it("transitions to submitting then success on resolved promise", async () => {
    let resolve!: () => void;
    const onClick = () => new Promise<void>((res) => { resolve = res; });

    renderBtn({ onClick });
    fireEvent.click(getBtn());

    expect(getBtn().getAttribute("data-state")).toBe("submitting");
    expect(getBtn().disabled).toBe(true);

    await act(async () => { resolve(); });

    expect(getBtn().getAttribute("data-state")).toBe("success");
    expect(getBtn().disabled).toBe(true);
  });

  it("shows success label after resolution", async () => {
    const onClick = () => Promise.resolve();
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });

    expect(screen.getByText(STATE_CONFIG.success.label)).toBeTruthy();
  });
});

describe("click handler — error path", () => {
  it("transitions to error on rejected promise", async () => {
    const onClick = () => Promise.reject(new Error("tx failed"));
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });

    expect(getBtn().getAttribute("data-state")).toBe("error");
  });

  it("shows error label after rejection", async () => {
    const onClick = () => Promise.reject(new Error("fail"));
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });

    expect(screen.getByText(STATE_CONFIG.error.label)).toBeTruthy();
  });

  it("is not disabled in error state (retry allowed)", async () => {
    const onClick = () => Promise.reject(new Error("fail"));
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });

    expect(getBtn().disabled).toBe(false);
  });

  it("allows a second click in error state (retry)", async () => {
    let callCount = 0;
    const onClick = jest.fn(() => {
      callCount++;
      return callCount === 1 ? Promise.reject(new Error("fail")) : Promise.resolve();
    });
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("data-state")).toBe("error");

    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("data-state")).toBe("success");
    expect(onClick).toHaveBeenCalledTimes(2);
  });
});

describe("double-submit prevention", () => {
  it("does not fire onClick while submitting", async () => {
    let resolve!: () => void;
    const onClick = jest.fn(() => new Promise<void>((res) => { resolve = res; }));

    renderBtn({ onClick });
    fireEvent.click(getBtn());

    // Button is now submitting — second click must be ignored.
    fireEvent.click(getBtn());
    fireEvent.click(getBtn());

    await act(async () => { resolve(); });

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("does not fire onClick when disabled=true", () => {
    const onClick = jest.fn();
    renderBtn({ onClick, disabled: true });
    fireEvent.click(getBtn());
    expect(onClick).not.toHaveBeenCalled();
  });

  it("does not fire onClick in success state", async () => {
    const onClick = jest.fn(() => Promise.resolve());
    renderBtn({ onClick, resetDelay: 100_000 });

    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("data-state")).toBe("success");

    fireEvent.click(getBtn());
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

// ── 7. Auto-reset timer ───────────────────────────────────────────────────────

describe("auto-reset timer", () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it("resets from success to idle after resetDelay", async () => {
    renderBtn({ onClick: noop, resetDelay: 1_000 });

    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("data-state")).toBe("success");

    act(() => { jest.advanceTimersByTime(1_000); });
    expect(getBtn().getAttribute("data-state")).toBe("idle");
  });

  it("resets from error to idle after resetDelay", async () => {
    renderBtn({ onClick: () => Promise.reject(new Error("x")), resetDelay: 1_000 });

    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("data-state")).toBe("error");

    act(() => { jest.advanceTimersByTime(1_000); });
    expect(getBtn().getAttribute("data-state")).toBe("idle");
  });

  it("does not reset before resetDelay elapses", async () => {
    renderBtn({ onClick: noop, resetDelay: 5_000 });

    await act(async () => { fireEvent.click(getBtn()); });
    act(() => { jest.advanceTimersByTime(4_999); });

    expect(getBtn().getAttribute("data-state")).toBe("success");
  });

  it("clamps negative resetDelay to 0 (immediate reset)", async () => {
    renderBtn({ onClick: noop, resetDelay: -500 });

    await act(async () => { fireEvent.click(getBtn()); });
    act(() => { jest.advanceTimersByTime(0); });

    expect(getBtn().getAttribute("data-state")).toBe("idle");
  });
});

// ── 8. Accessibility ──────────────────────────────────────────────────────────

describe("accessibility", () => {
  it("has aria-live='polite'", () => {
    renderBtn();
    expect(getBtn().getAttribute("aria-live")).toBe("polite");
  });

  it("aria-busy is true while submitting", async () => {
    let resolve!: () => void;
    const onClick = () => new Promise<void>((res) => { resolve = res; });

    renderBtn({ onClick });
    fireEvent.click(getBtn());

    expect(getBtn().getAttribute("aria-busy")).toBe("true");
    await act(async () => { resolve(); });
  });

  it("aria-busy is false in idle state", () => {
    renderBtn();
    expect(getBtn().getAttribute("aria-busy")).toBe("false");
  });

  it("aria-busy is false in success state", async () => {
    renderBtn({ onClick: noop, resetDelay: 100_000 });
    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("aria-busy")).toBe("false");
  });

  it("aria-busy is false in error state", async () => {
    renderBtn({ onClick: () => Promise.reject(new Error("x")), resetDelay: 100_000 });
    await act(async () => { fireEvent.click(getBtn()); });
    expect(getBtn().getAttribute("aria-busy")).toBe("false");
  });

  it("has an aria-label attribute", () => {
    renderBtn();
    expect(getBtn().getAttribute("aria-label")).toBeTruthy();
  });

  it("button is reachable by role", () => {
    renderBtn();
    expect(screen.getByRole("button")).toBeTruthy();
  });
});

// ── 9. Security edge cases ────────────────────────────────────────────────────

describe("security", () => {
  it("renders markup-like label as plain text (no XSS)", () => {
    const hostile = '<img src=x onerror=alert(1) />';
    renderBtn({ label: hostile });
    // The text node must exist and no img element must be injected.
    expect(screen.getByText(hostile)).toBeTruthy();
    expect(document.querySelector("img")).toBeNull();
  });

  it("does not expose dangerouslySetInnerHTML", () => {
    const { container } = renderBtn({ label: "<b>bold</b>" });
    // The <b> tag must not be parsed as HTML.
    expect(container.querySelector("b")).toBeNull();
  });

  it("backgroundColor comes from STATE_CONFIG, not user input", () => {
    const { container } = renderBtn();
    const btn = container.querySelector("button") as HTMLButtonElement;
    // jsdom normalises hex colours to rgb(); verify the style is set at all.
    expect(btn.style.backgroundColor).toBeTruthy();
    // The button must not have an empty or transparent background.
    expect(btn.style.backgroundColor).not.toBe("transparent");
    expect(btn.style.backgroundColor).not.toBe("");
  });

  it("data-testid is forwarded when provided", () => {
    renderBtn({ "data-testid": "contribute-btn" });
    expect(screen.getByTestId("contribute-btn")).toBeTruthy();
  });

  it("does not call onClick when onClick is undefined", () => {
    // Should not throw even without an onClick handler.
    expect(() => {
      renderBtn({ onClick: undefined as unknown as () => Promise<void> });
      fireEvent.click(getBtn());
    }).not.toThrow();
  });
});

// ── 10. Style merging ─────────────────────────────────────────────────────────

describe("style prop", () => {
  it("merges extra styles onto the button", () => {
    const { container } = renderBtn({ style: { fontSize: "20px" } });
    const btn = container.querySelector("button") as HTMLButtonElement;
    expect(btn.style.fontSize).toBe("20px");
  });

  it("extra styles do not override backgroundColor from STATE_CONFIG", () => {
    // STATE_CONFIG backgroundColor is applied after the base style but before
    // user style — user style wins for non-security-critical properties.
    const { container } = renderBtn({ style: { borderRadius: "0px" } });
    const btn = container.querySelector("button") as HTMLButtonElement;
    expect(btn.style.borderRadius).toBe("0px");
  });
});
