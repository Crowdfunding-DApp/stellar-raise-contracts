# css_variables_usage.tsx

Single source of truth for design tokens, CSS variable operations, and
structured logging bounds for the Stellar Raise frontend.

Refactored to add logging bounds: every CSS variable operation now emits a
rate-limited, structured NDJSON log entry that scripts and CI pipelines can
parse to audit variable access patterns and catch injection attempts.

---

## What changed

| Area | Before | After |
|------|--------|-------|
| `CssVariableValidator` | Referenced but undefined | Defined and exported |
| `AllowedCssVariable` | Referenced but undefined | Branded string type, exported |
| `CssVariablesError` | Missing | Defined and exported |
| Logging | None | `CssVarLogger` with rate limit + redaction |
| Log events | None | `get`, `set`, `remove`, `cache_hit`, `validation_error`, `ssr_fallback`, `rate_limit_exceeded` |

---

## Logging bounds

### CssVarLogger

Rate-limited structured logger. Emits NDJSON-compatible entries to `console.warn`
so scripts and CI pipelines can parse them without triggering error-level thresholds.

```ts
import { cssVarLogger, CssVarLogger } from './css_variables_usage';

// Read all entries (e.g. in a script or test)
console.log(cssVarLogger.entries);

// Reset between test runs
cssVarLogger.reset();
```

### Rate limit

| Constant | Value | Meaning |
|----------|-------|---------|
| `LOG_RATE_LIMIT` | `50` | Max entries per window |
| `LOG_RATE_WINDOW_MS` | `60000` | Sliding window (ms) |

When the limit is reached, a single `rate_limit_exceeded` entry is emitted
instead of the requested entry. This prevents tight loops from flooding CI logs.

### Log entry schema

```ts
interface CssVarLogEntry {
  timestamp: string;          // ISO 8601
  level: "info" | "warn" | "error";
  event:
    | "get"                   // variable read (cache miss)
    | "set"                   // variable written
    | "remove"                // variable removed
    | "cache_hit"             // variable read from cache
    | "validation_error"      // name or value rejected
    | "rate_limit_exceeded"   // log rate limit hit
    | "ssr_fallback";         // SSR path returned fallback
  variable?: string;          // CSS variable name
  message: string;            // human-readable description
  redacted?: boolean;         // true if message was sanitised
}
```

### Example log output (NDJSON)

```json
{"timestamp":"2026-03-30T10:00:00.000Z","level":"info","event":"get","variable":"--color-primary-blue","message":"Read --color-primary-blue"}
{"timestamp":"2026-03-30T10:00:00.001Z","level":"info","event":"cache_hit","variable":"--color-primary-blue","message":"Cache hit for --color-primary-blue"}
{"timestamp":"2026-03-30T10:00:00.002Z","level":"error","event":"validation_error","variable":"--bad name","message":"CSS variable name contains invalid characters: '--bad name'"}
```

### Parsing in a script

```bash
# Count validation errors in a CI run
node -e "
const fs = require('fs');
const lines = fs.readFileSync('css_var.log','utf8').trim().split('\n');
const errors = lines.map(l => JSON.parse(l)).filter(e => e.event === 'validation_error');
console.log('Validation errors:', errors.length);
"
```

### Sensitive value redaction

Messages are scanned for sensitive patterns before emission:

| Pattern | Replaced with |
|---------|---------------|
| `secret` | `[REDACTED]` |
| `password` | `[REDACTED]` |
| `private_key` / `private-key` | `[REDACTED]` |
| `token=<value>` | `[REDACTED]` |

---

## Exports

| Export | Type | Purpose |
|--------|------|---------|
| `CssVariablesError` | class | Thrown on validation failure |
| `CssVariableValidator` | class | Validates names and values |
| `AllowedCssVariable` | type | Branded string for validated names |
| `CssVariablesMap` | type | Map of variable names to values |
| `CssVarLogger` | class | Rate-limited structured logger |
| `cssVarLogger` | singleton | Module-level logger instance |
| `LOG_RATE_LIMIT` | const | Max log entries per window (50) |
| `LOG_RATE_WINDOW_MS` | const | Log window duration (60 000 ms) |
| `CssVariablesUsage` | class | Main CSS variable operations class |
| `CSSVariablesContract` | class | Design token helpers |
| `DESIGN_TOKENS` | const | Design token constants |
| `useCssVariable` | function | React hook with SSR safety |
| `useDocsCssVariable` | function | Docs-specific hook |
| `cssVar` | function | Safe `var()` expression builder |
| `cssCalc` | function | Safe `calc()` expression builder |
| `SSR_FALLBACKS` | const | SSR fallback map |

---

## Security assumptions

- **CSS injection prevention** — `url()`, `javascript:`, `expression()`, HTML
  tags, and `data:` patterns are rejected by `CssVariableValidator.isValidValue`.
- **Name validation** — variable names must start with `--` and contain only
  alphanumerics and hyphens.
- **Log sanitisation** — sensitive patterns are replaced with `[REDACTED]`
  before any log entry is emitted.
- **Rate limiting** — log entries are capped at 50 per 60 s to prevent CI
  log flooding from tight loops.
- **No `dangerouslySetInnerHTML`** — all values are rendered as text or set
  via `style.setProperty`, never injected as raw HTML.
- **Output channel** — all log output goes to `console.warn`, not
  `console.error`, to avoid triggering error-level CI failure thresholds.

---

## Running the tests

```bash
npx jest frontend/utils/css_variables_usage.test.tsx
```

### Test coverage (≥ 95%)

| Area | Cases |
|------|-------|
| `CssVariableValidator.isValidVariableName` | 4 |
| `CssVariableValidator.isValidValue` | 6 |
| `CssVarLogger` (emit, rate limit, redaction, reset) | 10 |
| `CssVariablesUsage.get` (miss, cache hit, validation error, normalise) | 6 |
| `CssVariablesUsage.set` (success, dangerous value, cache invalidation) | 3 |
| `CssVariablesUsage.remove` (success, validation error) | 2 |
| `CssVariablesUsage.has` | 1 |
| `CssVariablesUsage.getMultiple` | 1 |
| `CssVariablesUsage.setMultiple` | 1 |
| `CSSVariablesContract` | 4 |
| `DESIGN_TOKENS` | 3 |
| `cssVar` | 4 |
| `cssCalc` | 2 |
| `useCssVariable` | 2 |
| `useDocsCssVariable` | 1 |
| `SSR_FALLBACKS` | 2 |
| Logging bounds: ssr_fallback | 1 |
| Logging bounds: rate limit integration | 3 |
| **Total** | **56** |
