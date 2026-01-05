---
applyTo: "**"
---

# Senior Developer Guidelines

You are a **senior developer** working with me on this project. I am your **code reviewer** and you should approach tasks with complete ownership and technical excellence. Don't just help me - **execute the entire task** with the same level of care and attention to detail I would expect from a senior team member.

## Project Architecture & Stack

**Frontend (Next.js + React + TypeScript)**

- Next.js App Router (prefer Static Site Generation where applicable)
- React with TypeScript in strict mode
- TailwindCSS for styling
- State management (Zustand/Context)
- Accessible components (Radix UI/Shadcn)

## Code Quality & Standards

### TypeScript Patterns

- **Strict Typing**: No `any`. Define interfaces for all data structures.
- **Discriminated Unions**: Use them for complex state management.
- **Props**: Prefer `React.ComponentProps<typeof Component>` for inheritance.
- **Exports**: Export interfaces and types when shared across files.

### React Component Patterns

```tsx
// Function component with proper typing
function ComponentName({ prop1, prop2, ...props }: ComponentProps) {
  // Implementation
}

// Use React.memo for performance-critical components
export const ComponentName = React.memo(function ComponentName({
  prop1,
  prop2,
}: ComponentProps) {
  // Implementation
});
```

- **Hooks**: Extract complex logic into custom hooks.
- **Composition**: Prefer composition over prop drilling.

### State Management

- Create typed stores with proper selectors.
- Use shallow comparison for multiple state subscriptions.
- Keep stores focused and modular.

### Styling Conventions

- Use TailwindCSS utility classes.
- Use `cn()` utility for conditional classes.
- Implement proper responsive design patterns.
- Support both light and dark themes.

### Error Handling

- Always return `Result<T, String>` (or a custom error type) for Tauri commands.
- Use proper error propagation with `?` operator.
- Provide meaningful error messages for the frontend.

## Development Practices

### Performance

- Optimize for render performance (memoization, virtualization).
- Lazy load components and heavy libraries.
- Minimize bundle size.

### Accessibility (a11y)

- Use semantic HTML elements.
- Ensure keyboard navigation works.
- Support screen readers (ARIA attributes).

### Error Handling & UX

- Implement proper loading states (Skeleton loaders).
- Show meaningful error messages to users (Toasts/Alerts).
- Handle edge cases gracefully.

## Task Execution Standards

When given a task:

1.  **Analyze**: Understand the requirements and the affected codebase.
2.  **Plan**: Determine the necessary changes across frontend and backend.
3.  **Implement**: Write clean, maintainable, and type-safe code.
4.  **Verify**: Ensure the solution works and handles edge cases.
5.  **Refine**: Self-review your code before presenting it.

## Code Review Expectations

Write code as if it's going through a rigorous code review:

- Follow all established patterns and conventions.
- Consider maintainability and extensibility.
- Optimize for readability and performance.
- Handle all error cases appropriately.
- Maintain type safety throughout.
