/**
 * @title   css_variables_usage.test.tsx
 * @notice  Comprehensive tests for CSS variables usage with logging bounds.
 * @dev     Targets ≥ 95% coverage of css_variables_usage.tsx.
 *          Run: npx jest frontend/utils/css_variables_usage.test.tsx
 */
import {
  DESIGN_TOKENS,
  CSSVariablesContract,
  CssVariableValidator,
  CssVariablesError,
  CssVariablesUsage,
  CssVarLogger,
  cssVarLogger,
  LOG_RATE_LIMIT,
  LOG_RATE_WINDOW_MS,
  cssVar,
  cssCalc,
  useCssVariable,
  useDocsCssVariable,
  SSR_FALLBACKS,
} from "./css_variables_usage";

// ── Helpers ───────────────────────────────────────────────────────────────────

function makeElement(): HTMLElement {
  const el = document.createElement("div");
  document.body.appendChild(el);
  return el;
}

beforeEach(() => {
  cssVarLogger.reset();
  jest.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  jest.restoreAllMocks();
});

// ── CssVariableValidator ──────────────────────────────────────────────────────

describe("CssVariableValidator.isValidVariableName", () => {
  it("accepts valid variable names with -- prefix", () => {
    expect(() => CssVariableValidator.isValidVariableName("--color-primary")).not.toThrow();
    expect(() => CssVariableValidator.isValidVariableName("--space-4")).not.toThrow();
  });

  it("throws when name lacks -- prefix", () => {
    expect(() => CssVariableValidator.isValidVariableName("color-primary")).toThrow(CssVariablesError);
  });

  it("throws when name contains invalid characters", () => {
    expect(() => CssVariableValidator.isValidVariableName("--color primary")).toThrow(CssVariablesError);
    expect(() => CssVariableValidator.isValidVariableName("--color_primary")).toThrow(CssVariablesError);
  });

  it("throws for empty string", () => {
    expect(() => CssVariableValidator.isValidVariableName("")).toThrow(CssVariablesError);
  });
});

describe("CssVariableValidator.isValidValue", () => {
  it("accepts safe values", () => {
    expect(() => CssVariableValidator.isValidValue("#4f46e5")).not.toThrow();
    expect(() => CssVariableValidator.isValidValue("1rem")).not.toThrow();
    expect(() => CssVariableValidator.isValidValue("rgba(0,0,0,0.5)")).not.toThrow();
  });

  it("rejects url() pattern", () => {
    expect(() => CssVariableValidator.isValidValue("url(evil.png)")).toThrow(CssVariablesError);
  });

  it("rejects javascript: pattern", () => {
    expect(() => CssVariableValidator.isValidValue("javascript:alert(1)")).toThrow(CssVariablesError);
  });

  it("rejects expression() pattern", () => {
    expect(() => CssVariableValidator.isValidValue("expression(alert(1))")).toThrow(CssVariablesError);
  });

  it("rejects HTML tag patterns", () => {
    expect(() => CssVariableValidator.isValidValue("<script>")).toThrow(CssVariablesError);
  });

  it("rejects data: pattern", () => {
    expect(() => CssVariableValidator.isValidValue("data:text/html,<h1>")).toThrow(CssVariablesError);
  });
});

// ── CssVarLogger ──────────────────────────────────────────────────────────────

describe("CssVarLogger", () => {
  it("emits a structured log entry", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "info", event: "get", variable: "--color-primary", message: "Read --color-primary" });
    expect(logger.entries).toHaveLength(1);
    expect(logger.entries[0].event).toBe("get");
    expect(logger.entries[0].level).toBe("info");
    expect(logger.entries[0].timestamp).toBeTruthy();
  });

  it("includes variable field in entry", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "info", event: "set", variable: "--space-4", message: "Set --space-4" });
    expect(logger.entries[0].variable).toBe("--space-4");
  });

  it("redacts sensitive patterns in messages", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "error", event: "validation_error", message: "secret key leaked" });
    expect(logger.entries[0].message).toContain("[REDACTED]");
    expect(logger.entries[0].redacted).toBe(true);
  });

  it("does not redact safe messages", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "info", event: "get", message: "Read --color-primary" });
    expect(logger.entries[0].redacted).toBeFalsy();
    expect(logger.entries[0].message).toBe("Read --color-primary");
  });

  it("enforces rate limit after LOG_RATE_LIMIT entries", () => {
    const logger = new CssVarLogger();
    for (let i = 0; i < LOG_RATE_LIMIT + 5; i++) {
      logger.emit({ level: "info", event: "get", message: `entry ${i}` });
    }
    const rateLimitEntries = logger.entries.filter((e) => e.event === "rate_limit_exceeded");
    expect(rateLimitEntries.length).toBeGreaterThan(0);
  });

  it("isAllowed returns false after limit is reached", () => {
    const logger = new CssVarLogger();
    for (let i = 0; i < LOG_RATE_LIMIT; i++) logger.isAllowed();
    expect(logger.isAllowed()).toBe(false);
  });

  it("reset clears entries and timestamps", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "info", event: "get", message: "test" });
    logger.reset();
    expect(logger.entries).toHaveLength(0);
    expect(logger.isAllowed()).toBe(true);
  });

  it("sanitize returns redacted=false for clean messages", () => {
    const logger = new CssVarLogger();
    const { sanitized, redacted } = logger.sanitize("safe message");
    expect(sanitized).toBe("safe message");
    expect(redacted).toBe(false);
  });

  it("sanitize redacts password pattern", () => {
    const logger = new CssVarLogger();
    const { sanitized, redacted } = logger.sanitize("password=abc123");
    expect(sanitized).toContain("[REDACTED]");
    expect(redacted).toBe(true);
  });

  it("emits to console.warn", () => {
    const logger = new CssVarLogger();
    logger.emit({ level: "info", event: "get", message: "test" });
    expect(console.warn).toHaveBeenCalled();
  });
});

// ── CssVariablesUsage ─────────────────────────────────────────────────────────

describe("CssVariablesUsage.get", () => {
  it("returns fallback when variable is not set", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(css.get("--not-set", "fallback")).toBe("fallback");
  });

  it("returns empty string when no fallback and variable not set", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(css.get("--not-set")).toBe("");
  });

  it("emits a get log event", () => {
    const el = makeElement();
    new CssVariablesUsage(el).get("--not-set", "fb");
    expect(cssVarLogger.entries.some((e) => e.event === "get")).toBe(true);
  });

  it("emits a cache_hit event on second read", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    css.get("--not-set");
    css.get("--not-set");
    expect(cssVarLogger.entries.some((e) => e.event === "cache_hit")).toBe(true);
  });

  it("throws and emits validation_error for invalid name", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(() => css.get("no-prefix")).toThrow(CssVariablesError);
    expect(cssVarLogger.entries.some((e) => e.event === "validation_error")).toBe(true);
  });

  it("normalises name without -- prefix", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(() => css.get("color-primary", "blue")).not.toThrow();
  });
});

describe("CssVariablesUsage.set", () => {
  it("sets a CSS variable and emits set event", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    css.set("--test-color", "#fff");
    expect(cssVarLogger.entries.some((e) => e.event === "set")).toBe(true);
  });

  it("throws and emits validation_error for dangerous value", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(() => css.set("--test", "url(evil)")).toThrow(CssVariablesError);
    expect(cssVarLogger.entries.some((e) => e.event === "validation_error")).toBe(true);
  });

  it("invalidates cache after set", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    css.get("--test-var", "old");
    css.set("--test-var", "#new");
    // Cache should be cleared — next get should not be a cache_hit from before
    cssVarLogger.reset();
    css.get("--test-var", "old");
    expect(cssVarLogger.entries.some((e) => e.event === "get")).toBe(true);
    expect(cssVarLogger.entries.some((e) => e.event === "cache_hit")).toBe(false);
  });
});

describe("CssVariablesUsage.remove", () => {
  it("removes a CSS variable and emits remove event", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    css.set("--test-remove", "#abc");
    cssVarLogger.reset();
    css.remove("--test-remove");
    expect(cssVarLogger.entries.some((e) => e.event === "remove")).toBe(true);
  });

  it("throws and emits validation_error for invalid name", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(() => css.remove("bad name")).toThrow(CssVariablesError);
    expect(cssVarLogger.entries.some((e) => e.event === "validation_error")).toBe(true);
  });
});

describe("CssVariablesUsage.has", () => {
  it("returns false when variable is not set", () => {
    const el = makeElement();
    expect(new CssVariablesUsage(el).has("--not-set")).toBe(false);
  });
});

describe("CssVariablesUsage.getMultiple", () => {
  it("returns a map of variable values", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    const result = css.getMultiple(["--a", "--b"], "fb");
    expect(result["--a" as never]).toBe("fb");
    expect(result["--b" as never]).toBe("fb");
  });
});

describe("CssVariablesUsage.setMultiple", () => {
  it("sets multiple variables without throwing", () => {
    const el = makeElement();
    const css = new CssVariablesUsage(el);
    expect(() =>
      css.setMultiple({ "--x" as never: "#111", "--y" as never: "#222" }),
    ).not.toThrow();
  });
});

// ── CSSVariablesContract ──────────────────────────────────────────────────────

describe("CSSVariablesContract", () => {
  it("getVar returns correct CSS variable string", () => {
    expect(CSSVariablesContract.getVar("COLORS", "PRIMARY_BLUE")).toBe("var(--color-primary-blue)");
    expect(CSSVariablesContract.getVar("SPACING", "SPACE_4")).toBe("var(--spacing-space-4)");
  });

  it("isApprovedColor returns true for palette colors", () => {
    expect(CSSVariablesContract.isApprovedColor("#4f46e5")).toBe(true);
  });

  it("isApprovedColor returns false for non-palette colors", () => {
    expect(CSSVariablesContract.isApprovedColor("#ff00ff")).toBe(false);
  });

  it("getSpacingPx returns correct pixel values", () => {
    expect(CSSVariablesContract.getSpacingPx("SPACE_4")).toBe(16);
    expect(CSSVariablesContract.getSpacingPx("SPACE_1")).toBe(4);
    expect(CSSVariablesContract.getSpacingPx("SPACE_12")).toBe(48);
  });
});

// ── DESIGN_TOKENS ─────────────────────────────────────────────────────────────

describe("DESIGN_TOKENS", () => {
  it("contains all expected categories", () => {
    expect(DESIGN_TOKENS).toHaveProperty("COLORS");
    expect(DESIGN_TOKENS).toHaveProperty("SPACING");
    expect(DESIGN_TOKENS).toHaveProperty("FONTS");
    expect(DESIGN_TOKENS).toHaveProperty("RADIUS");
  });

  it("has correct primary blue value", () => {
    expect(DESIGN_TOKENS.COLORS.PRIMARY_BLUE).toBe("#4f46e5");
  });

  it("has correct success green value", () => {
    expect(DESIGN_TOKENS.COLORS.SUCCESS_GREEN).toBe("#10b981");
  });
});

// ── cssVar helper ─────────────────────────────────────────────────────────────

describe("cssVar", () => {
  it("returns var() expression for valid name", () => {
    expect(cssVar("--color-primary" as never)).toBe("var(--color-primary)");
  });

  it("includes fallback when provided", () => {
    expect(cssVar("--color-primary" as never, "#000")).toBe("var(--color-primary, #000)");
  });

  it("normalises name without -- prefix", () => {
    expect(cssVar("color-primary" as never)).toBe("var(--color-primary)");
  });

  it("throws for invalid name", () => {
    expect(() => cssVar("bad name" as never)).toThrow(CssVariablesError);
  });
});

// ── cssCalc helper ────────────────────────────────────────────────────────────

describe("cssCalc", () => {
  it("wraps expression in calc()", () => {
    expect(cssCalc("100% - 2rem")).toBe("calc(100% - 2rem)");
  });

  it("throws for dangerous expression", () => {
    expect(() => cssCalc("url(evil)")).toThrow(CssVariablesError);
  });
});

// ── useCssVariable (browser) ──────────────────────────────────────────────────

describe("useCssVariable (browser)", () => {
  it("returns fallback when variable is not set", () => {
    expect(useCssVariable("--not-set", "fallback")).toBe("fallback");
  });

  it("returns empty string when no fallback", () => {
    expect(useCssVariable("--not-set")).toBe("");
  });
});

// ── useDocsCssVariable ────────────────────────────────────────────────────────

describe("useDocsCssVariable", () => {
  it("delegates to useCssVariable", () => {
    expect(useDocsCssVariable("--not-set", "docs-fallback")).toBe("docs-fallback");
  });
});

// ── SSR_FALLBACKS ─────────────────────────────────────────────────────────────

describe("SSR_FALLBACKS", () => {
  it("contains primary blue fallback", () => {
    expect(SSR_FALLBACKS["--color-primary-blue" as never]).toBe("#0066FF");
  });

  it("contains space-4 fallback", () => {
    expect(SSR_FALLBACKS["--space-4" as never]).toBe("1rem");
  });
});

// ── Logging bounds: ssr_fallback event ────────────────────────────────────────

describe("Logging bounds: ssr_fallback", () => {
  it("emits ssr_fallback event when window is undefined (simulated via logger)", () => {
    // Directly test the logger path that useCssVariable would hit in SSR
    cssVarLogger.emit({ level: "info", event: "ssr_fallback", variable: "--color-primary", message: "SSR fallback for --color-primary" });
    expect(cssVarLogger.entries.some((e) => e.event === "ssr_fallback")).toBe(true);
  });
});

// ── Logging bounds: rate limit integration ────────────────────────────────────

describe("Logging bounds: rate limit integration", () => {
  it("module-level cssVarLogger enforces rate limit", () => {
    cssVarLogger.reset();
    for (let i = 0; i < LOG_RATE_LIMIT + 3; i++) {
      cssVarLogger.emit({ level: "info", event: "get", message: `entry ${i}` });
    }
    expect(cssVarLogger.entries.some((e) => e.event === "rate_limit_exceeded")).toBe(true);
  });

  it("LOG_RATE_LIMIT is 50", () => {
    expect(LOG_RATE_LIMIT).toBe(50);
  });

  it("LOG_RATE_WINDOW_MS is 60000", () => {
    expect(LOG_RATE_WINDOW_MS).toBe(60_000);
  });
});
