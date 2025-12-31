# Comment Style Guide

This guide establishes consistent comment styles across the Mindia codebase for better maintainability and developer experience.

## Table of Contents

- [Rust](#rust)
- [TypeScript/JavaScript](#typescriptjavascript)
- [Java](#java)
- [General Guidelines](#general-guidelines)
- [Special Comment Tags](#special-comment-tags)

---

## Rust

### Module-Level Documentation

Use `//!` for crate and module-level documentation:

```rust
//! Module name
//!
//! Brief description of what this module provides.
//! Can span multiple lines.
//!
//! Additional details about the module's purpose, design decisions, etc.
```

**Example:**
```rust
//! Media repository module
//!
//! This module provides database access for media files (images, videos, etc.)
//! and coordinates with storage backends.
```

### Item-Level Documentation

Use `///` for documenting public items (functions, structs, enums, traits, etc.):

```rust
/// Brief one-line description.
///
/// Longer description if needed. Can include:
/// - Bullet points
/// - Multiple paragraphs
///
/// # Arguments
/// * `param1` - Description of parameter 1
/// * `param2` - Description of parameter 2
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When and why this function returns an error
///
/// # Examples
/// ```
/// let result = example_function(arg1, arg2)?;
/// ```
pub fn example_function(param1: Type1, param2: Type2) -> Result<ReturnType, Error> {
    // Implementation
}
```

**Key Points:**
- First line should be a brief summary (used by rustdoc)
- Use `# Arguments`, `# Returns`, `# Errors`, `# Examples` sections as needed
- Always document public APIs
- Use code blocks in examples

### Inline Comments

Use `//` for inline comments explaining implementation details:

```rust
// Explain why, not what
let result = complex_calculation();

// For complex logic, use multi-line comments
// This algorithm uses a two-pass approach:
// 1. First pass collects all candidates
// 2. Second pass filters and ranks them
let filtered = two_pass_algorithm(data);
```

**Guidelines:**
- Place comments on separate lines above the code
- Explain "why" for non-obvious decisions
- Avoid obvious comments that just restate the code
- Use comments to explain complex algorithms or business logic

### Section Separators

For large files, use consistent section separators:

```rust
// =============================================================================
// SECTION NAME
// =============================================================================
```

**Example:**
```rust
// =============================================================================
// IMAGE OPERATIONS
// =============================================================================
```

---

## TypeScript/JavaScript

### File-Level Documentation

Add a file header comment for service files and major modules:

```typescript
/**
 * Service/module name
 *
 * Brief description of what this file provides.
 */
```

**Example:**
```typescript
/**
 * Images service
 *
 * Provides methods for uploading, managing, and transforming images.
 */
```

### Class Documentation

Document classes with JSDoc:

```typescript
/**
 * Class description
 *
 * Additional details about the class's purpose and usage.
 */
export class ExampleService {
    // ...
}
```

### Method Documentation

Use JSDoc for all public methods:

```typescript
/**
 * Method description
 *
 * Longer description if needed. Can include details about behavior,
 * edge cases, or important notes.
 *
 * @param param1 - Parameter description
 * @param param2 - Optional parameter description
 * @param options - Options object with properties:
 *   - `option1`: Description
 *   - `option2`: Description
 * @returns Return value description
 * @throws {ErrorType} When and why this error occurs
 *
 * @example
 * ```typescript
 * const result = await service.method(param1, param2, {
 *   option1: 'value',
 *   option2: true
 * });
 * ```
 */
async method(param1: string, param2?: number, options?: Options): Promise<Result> {
    // Implementation
}
```

**Key Points:**
- Use `@param` for all parameters
- Use `@returns` (not `@return`) for return values
- Use `@throws {ErrorType}` with the error type
- Add `@example` for complex APIs
- Use `-` after parameter names for descriptions

### Inline Comments

Use `//` for inline comments:

```typescript
// Explain why this approach is used
const result = await complexOperation();

// For complex logic, use multi-line comments
// This handles the edge case where the API might return
// a cached response that needs to be invalidated
if (needsInvalidation) {
    await invalidateCache();
}
```

### Type Definitions

Document complex types and interfaces:

```typescript
/**
 * Configuration options
 *
 * @property baseUrl - API base URL
 * @property timeout - Request timeout in milliseconds
 * @property retries - Number of retry attempts
 */
export interface Config {
    baseUrl: string;
    timeout?: number;
    retries?: number;
}
```

---

## Java

### Class Documentation

Use JavaDoc for all public classes:

```java
/**
 * Class description
 *
 * Longer description if needed. Can include details about
 * the class's purpose, usage patterns, and important notes.
 */
public class ExampleService {
    // ...
}
```

### Method Documentation

Use JavaDoc for all public methods:

```java
/**
 * Method description
 *
 * Longer description if needed. Can include details about
 * behavior, edge cases, or important implementation notes.
 *
 * @param param1 Parameter description
 * @param param2 Optional parameter description
 * @return Return value description
 * @throws ExceptionType When and why this exception is thrown
 *
 * @since 1.0.0
 */
public ReturnType method(ParamType param1, OptionalType param2) throws ExceptionType {
    // Implementation
}
```

**Key Points:**
- First sentence should be a brief summary (used by JavaDoc)
- Use `@param` for all parameters
- Use `@return` (not `@returns`) for return values
- Use `@throws` for exceptions
- Add `@since` for version tracking when appropriate

### Inline Comments

Use `//` for inline comments:

```java
// Explain why this approach is used
Result result = complexOperation();

// For complex logic, use multi-line comments
// This handles the edge case where the API might return
// a cached response that needs to be invalidated
if (needsInvalidation) {
    invalidateCache();
}
```

### Complex Methods

For complex private methods, add JavaDoc even if they're not public:

```java
/**
 * Helper method that performs complex calculation
 *
 * @param input Input data
 * @return Calculated result
 */
private ResultType complexCalculation(InputType input) {
    // Implementation
}
```

---

## General Guidelines

### When to Comment

**DO comment:**
- Public APIs (functions, methods, classes)
- Complex algorithms or business logic
- Non-obvious design decisions ("why" not "what")
- Workarounds or temporary solutions
- Performance considerations
- Edge cases or gotchas

**DON'T comment:**
- Obvious code that is self-explanatory
- Code that should be refactored instead
- Outdated information (update or remove)

### Comment Quality

- **Be clear and concise**: Write comments that add value
- **Keep comments up to date**: Update comments when code changes
- **Use proper grammar**: Comments are part of the documentation
- **Explain why, not what**: The code shows what, comments explain why

### Code Examples

When providing examples in documentation:
- Use realistic examples
- Show common use cases
- Include error handling when relevant
- Keep examples simple and focused

---

## Special Comment Tags

### TODO Comments

Use consistent format for TODO items:

```rust
// TODO: Brief description of what needs to be done
// TODO(#123): Description with issue reference
```

```typescript
// TODO: Brief description
// TODO(#123): Description with issue reference
```

```java
// TODO: Brief description
// TODO(#123): Description with issue reference
```

**Guidelines:**
- Always use uppercase `TODO`
- Include issue reference when applicable
- Be specific about what needs to be done
- Don't leave TODOs without a plan to address them

### FIXME Comments

For code that needs to be fixed:

```rust
// FIXME: Brief description of the issue
// FIXME(#123): Description with issue reference
```

**Guidelines:**
- Use uppercase `FIXME`
- Include issue reference when applicable
- Explain what's wrong and what needs to be fixed

### NOTE Comments

For important notes or warnings:

```rust
// NOTE: Important information about this code
// NOTE: This uses a workaround for issue #123
```

```typescript
// NOTE: Important information
```

```java
// NOTE: Important information
```

**Guidelines:**
- Use uppercase `NOTE`
- Use for important information that developers should know
- Don't overuse - reserve for genuinely important notes

### HACK Comments

For temporary workarounds:

```rust
// HACK: Brief description of the workaround
// HACK: Temporary solution until library is updated
```

**Guidelines:**
- Use sparingly
- Explain why the hack is necessary
- Include a plan to remove it

### XXX Comments

For code that needs attention:

```rust
// XXX: This needs refactoring
// XXX: Performance issue - needs optimization
```

**Guidelines:**
- Use for code that needs significant attention
- Explain what the issue is

---

## Documentation Coverage

### Minimum Requirements

All of the following should have documentation:
- Public functions/methods
- Public classes/structs
- Public modules/packages
- Complex algorithms
- Non-obvious design decisions

### Code Review Checklist

When reviewing code, check:
- [ ] All public APIs are documented
- [ ] Complex logic has explanatory comments
- [ ] TODO/FIXME comments have issue references
- [ ] Comments are up to date with code
- [ ] Examples in documentation are correct
- [ ] Comment style follows this guide

---

## Examples

### Good Examples

**Rust:**
```rust
/// Calculates the optimal image dimensions while maintaining aspect ratio.
///
/// # Arguments
/// * `original_width` - Original image width in pixels
/// * `original_height` - Original image height in pixels
/// * `max_dimension` - Maximum width or height allowed
///
/// # Returns
/// Tuple of (width, height) that maintains aspect ratio
///
/// # Examples
/// ```
/// let (w, h) = calculate_dimensions(1920, 1080, 800);
/// assert_eq!(w, 800);
/// assert_eq!(h, 450);
/// ```
pub fn calculate_dimensions(
    original_width: u32,
    original_height: u32,
    max_dimension: u32,
) -> (u32, u32) {
    // Implementation
}
```

**TypeScript:**
```typescript
/**
 * Upload an image file
 *
 * @param file - Image file to upload
 * @param options - Upload options
 * @param options.store - Storage behavior ('0', '1', or 'auto')
 * @returns Uploaded image metadata
 * @throws {MindiaAPIError} If upload fails or file is invalid
 *
 * @example
 * ```typescript
 * const image = await imagesService.upload(file, {
 *   store: 'auto'
 * });
 * ```
 */
async upload(file: File, options?: { store?: StoreBehavior }): Promise<Image> {
    // Implementation
}
```

**Java:**
```java
/**
 * Upload an image file
 *
 * @param file Image file to upload
 * @param store Storage behavior ('0', '1', or 'auto')
 * @return Uploaded image metadata
 * @throws MindiaAPIException If upload fails or file is invalid
 */
public Image upload(File file, @Nullable String store) throws MindiaAPIException {
    // Implementation
}
```

### Bad Examples

**Don't do this:**
```rust
// This function does something
pub fn do_something() {
    // Set x to 5
    let x = 5;
    // Return x
    return x;
}
```

**Why it's bad:**
- Obvious comments that don't add value
- Comments that just restate the code
- Missing proper documentation format

---

## Tools and Automation

### Rust

- `cargo doc` - Generate documentation
- `cargo clippy` - Can check for missing documentation (with appropriate lints)

### TypeScript/JavaScript

- ESLint with JSDoc rules
- TypeDoc for generating documentation

### Java

- JavaDoc tool for generating documentation
- Checkstyle or similar for enforcing JavaDoc rules

---

## Questions?

If you're unsure about comment style:
1. Check this guide
2. Look at similar code in the codebase
3. Ask in code review
4. Follow language-specific conventions (rustdoc, JSDoc, JavaDoc)

---

## Updates

This guide should be updated as:
- New patterns emerge
- Language-specific tools change
- Team conventions evolve

Last updated: 2024
