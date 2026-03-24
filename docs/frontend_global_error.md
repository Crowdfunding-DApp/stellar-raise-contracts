# Frontend Global Error Boundary

Technical reference for the React global error boundary built for the Stellar Raise frontend.

---

## Overview

`FrontendGlobalErrorBoundary` is a React class component that catches synchronous render-phase errors anywhere in its wrapped component tree. It prevents full application crashes, classifies errors as generic or smart-contract related, and renders an appropriate fallback UI with a recovery path.

```
Error thrown → getDerivedStateFromError (state) →
componentDidCatch (logging + onError callback) → fallback UI
```

---

## Component API

```tsx
import {
  FrontendGlobalErrorBoundary,
  ContractError,
  NetworkError,
  TransactionError,
} from '../components/frontend_global_error';
```

### Props

| Prop | Type | Required | Description |
|------|------|----------|-------------|
| `children` | `ReactNode` | No | Component tree to protect |
| `fallback` | `ReactNode` | No | Custom fallback UI; overrides built-in fallback entirely |
| `onError` | `(report: ErrorReport) => void` | No | Callback invoked with a sanitised error report on every caught error |

### ErrorReport shape

```ts
interface ErrorReport {
  message: string;
  stack: string | undefined;        // omitted in production
  componentStack: string | undefined; // omitted in production
  timestamp: string;                // ISO 8601
  isSmartContractError: boolean;
  errorName: string;
}
```

---

## Custom Error Classes

Use these to signal specific failure domains to the boundary:

```tsx
// Smart contract execution failure
throw new ContractError('Insufficient funds for transaction');

// Network / Horizon API failure
throw new NetworkError('Horizon endpoint unreachable');

// Transaction signing / submission failure
throw new TransactionError('User rejected transaction in wallet');
```

All three extend `Error` and are automatically classified as smart-contract errors by the boundary.

---

## Error Classification

The boundary classifies an error as a smart-contract error when:

1. It is an instance of `ContractError`, `NetworkError`, or `TransactionError`.
2. Its `name` or `message` contains any of these keywords (case-insensitive):
   `contract`, `stellar`, `soroban`, `transaction`, `blockchain`, `ledger`,
   `horizon`, `xdr`, `invoke`, `wallet`.

All other errors render the generic "Documentation Loading Error" fallback.

---

## Fallback UIs

### Generic fallback
- ⚠️ icon
- Title: "Documentation Loading Error"
- "Try Again" and "Go Home" buttons

### Smart contract fallback
- 🔗 icon
- Title: "Smart Contract Error"
- Blockchain-specific guidance (wallet balance, connectivity)
- "Try Again" and "Go Home" buttons

### Dev-only error details
In `NODE_ENV !== 'production'`, a collapsible `<details>` element shows the raw error message to aid debugging. This section is hidden in production to prevent information disclosure.

---

## Usage

### Basic
# Frontend Global Error Boundary for Documentation

## Overview
The `FrontendGlobalErrorBoundary` is a React class component designed to catch runtime JavaScript errors within the Documentation section of the `stellar-raise-contracts` frontend application. By intercepting these errors, it prevents them from crashing the entire React component tree, ensuring that the rest of the application remains functional and displaying a user-friendly fallback UI.

## Usage
Wrap the generic components or specifically the documentation wrappers with the error boundary component.

```tsx
import { FrontendGlobalErrorBoundary } from '../components/frontend_global_error';

function App() {
  return (
    <FrontendGlobalErrorBoundary>
      <MainApplication />
function DocumentationPage() {
  return (
    <FrontendGlobalErrorBoundary>
      <DocumentationContent />
    </FrontendGlobalErrorBoundary>
  );
}
```

### With custom fallback

```tsx
<FrontendGlobalErrorBoundary fallback={<div>Custom error UI</div>}>
  <MainApplication />
</FrontendGlobalErrorBoundary>
```

### With error reporting (Sentry example)

```tsx
import * as Sentry from '@sentry/react';

<FrontendGlobalErrorBoundary
  onError={(report) => Sentry.captureMessage(report.message, { extra: report })}
>
  <MainApplication />
</FrontendGlobalErrorBoundary>
```

### Throwing typed errors in contract components

```tsx
import { ContractError } from '../components/frontend_global_error';

async function contribute(amount: number) {
  try {
    await contract.invoke('contribute', { amount });
  } catch (err) {
    throw new ContractError(`Contribution failed: ${(err as Error).message}`);
  }
}
```

---

## Security Considerations

| Concern | Mitigation |
|---------|-----------|
| Information disclosure | Stack traces and component stacks are omitted from `ErrorReport` in production |
| XSS via error messages | Fallback UI renders error message as React text node (not `innerHTML`) |
| Sensitive contract data | Custom error classes should never embed private keys, XDR, or account secrets in the message |
| Async errors | The boundary does NOT catch errors in event handlers, `setTimeout`, or SSR — handle those separately |

---

## Limitations

- Cannot catch errors thrown inside the boundary's own `render` method.
- Does not catch async errors (event handlers, `Promise` rejections, `setTimeout`).
- Does not catch server-side rendering errors (use Next.js `_error.tsx` / `500.tsx` for those).
- Nested boundaries can be used for more granular isolation of subsections.

---

## Test Coverage

Tests live in `frontend/components/frontend_global_error.test.tsx` and cover:

- Custom error class instantiation and inheritance
- Normal (no-error) rendering
- Generic error fallback rendering and logging
- Smart contract error detection (10 keyword/type variants)
- Custom fallback prop (generic and contract errors)
- Recovery via "Try Again" (success and persistent-error cases)
- `onError` callback with structured report validation
- Accessibility (`role="alert"`, `aria-live`, `aria-label`, `aria-hidden`)
- Edge cases: empty message, TypeError, keyword matching

Target: ≥ 95% statement and line coverage, 100% function coverage.

---

## Integration with Next.js

```tsx
// pages/_app.tsx
import GlobalErrorBoundary from '../components/frontend_global_error';

function MyApp({ Component, pageProps }) {
  return (
    <GlobalErrorBoundary>
      <Component {...pageProps} />
    </GlobalErrorBoundary>
  );
}
```

The boundary handles client-side render errors. `pages/500.tsx` handles server-side errors. Both should be present for full coverage.
### Custom Fallback Customization
If the default fallback UI is insufficient, you can provide a custom `fallback` node via props.

```tsx
<FrontendGlobalErrorBoundary fallback={<div>Custom documentation crash message.</div>}>
  <DocumentationContent />
</FrontendGlobalErrorBoundary>
```

## Security & Reliability Assumptions
- **XSS Prevention**: By catching unexpected UI errors, the boundary prevents any corrupted rendering states that might inadvertently expose raw sensitive debug information to the DOM. The fallback UI strictly relies on standard string interpolation preventing XSS.
- **Enhanced Isolation**: Documentation code heavily relies on rendering external or dense markdown content. The error boundary acts as a blast door isolating the core platform (like investment flows or onboarding) from documentation failures.
- **Fail-safe mechanism**: The boundary provides a built-in "Try Again" recovery mechanism. It clears the internal error state, prompting a re-render of the child component tree without needing a full page reload, improving UX.

## Testing and Coverage
The component is fully unit-tested using Jest and `@testing-library/react`. Tests encompass:
1. Standard non-error render validation.
2. Caught error validation with the default fallback and simulated console error logs.
3. Custom fallback render validation.
4. Error recovery ("Try Again" logic) flow validation ensuring state is properly updated. Minimum 95% test coverage is enforced.
