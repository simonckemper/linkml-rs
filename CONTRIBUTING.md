# Contributing to LinkML Rust

Thank you for your interest in contributing to the LinkML Rust implementation! This document provides guidelines and instructions for contributing to this project.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Making Changes](#making-changes)
5. [Coding Standards](#coding-standards)
6. [Testing Requirements](#testing-requirements)
7. [Documentation](#documentation)
8. [Pull Request Process](#pull-request-process)
9. [Release Process](#release-process)

## Code of Conduct

This project follows standard open-source community guidelines. We expect all contributors to:

- Be respectful and professional in all interactions
- Provide constructive feedback
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

- Rust 2024 edition or later
- Git
- Familiarity with LinkML concepts
- Understanding of Rust async/await patterns

### Finding Issues to Work On

- Check the [issue tracker](https://github.com/simonckemper/rootreal/issues) for open issues
- Look for issues labeled `good-first-issue` for beginner-friendly tasks
- Issues labeled `help-wanted` are prioritized for external contributions
- Feel free to propose new features or improvements

## Development Setup

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/rootreal.git
cd rootreal/crates/model/symbolic/linkml
```

### 2. Install Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
rustup component add clippy rustfmt

# Install cargo-tarpaulin for code coverage (optional)
cargo install cargo-tarpaulin
```

### 3. Build the Project

```bash
# Build all crates
cargo build --all-features

# Run tests to verify setup
cargo test --all-features
```

### 4. Set Up Pre-commit Hooks (Optional)

```bash
# Create a pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all-features

echo "✅ All checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/your-feature-name` for new features
- `fix/issue-description` for bug fixes
- `docs/documentation-update` for documentation changes
- `refactor/component-name` for refactoring

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `perf`: Performance improvements
- `chore`: Maintenance tasks

**Examples:**
```
feat(validator): add support for custom validation functions

fix(typeql): correct entity relationship generation for complex schemas

docs(readme): update installation instructions
```

## Coding Standards

### Rust Standards

This project follows strict RootReal coding standards:

#### Zero Tolerance Policy

- **No placeholders**: No `TODO`, `FIXME`, or `unimplemented!()` in production code
- **No mocks in production/examples**: Mocks only allowed in `#[cfg(test)]` modules
- **No unwrap()**: Use proper error handling with `Result` and `?`
- **No unsafe code**: Unless absolutely necessary and documented
- **Zero warnings**: All code must compile without warnings

#### Code Quality

- **Module Size**: Keep modules under 500 LOC (can exceed to 550 if splitting harms cohesion)
- **SOLID Principles**: Strict adherence to Single Responsibility Principle
- **DRY**: Don't Repeat Yourself
- **Dependency Injection**: Use factory functions, not direct instantiation
- **Async/Await**: Use structured concurrency via TaskManagementService

#### Rust 2024 Edition

Always use modern Rust 2024 features:

```rust
// ✅ Good: Use let chains
if let Some(x) = option && let Some(y) = other {
    // ...
}

// ✅ Good: Use is_some_and()
if option.is_some_and(|x| x > 10) {
    // ...
}

// ❌ Bad: Old patterns
if let Some(x) = option {
    if let Some(y) = other {
        // ...
    }
}
```

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Fix Clippy warnings automatically (where possible)
cargo clippy --fix --all-targets --all-features
```

## Testing Requirements

### Test Coverage

- **Unit tests**: >90% coverage required
- **Integration tests**: >80% coverage required
- **Business logic**: Tests must validate real business use cases

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_with_real_schema() {
        // Arrange
        let service = create_linkml_service().await.expect("Failed to create service");
        let schema = load_test_schema("person.yaml").await.expect("Failed to load schema");

        // Act
        let result = service.validate_data(
            &schema,
            &serde_json::json!({
                "name": "John Doe",
                "email": "john@example.com"
            }),
            "Person"
        ).await.expect("Validation failed");

        // Assert
        assert!(result.is_valid(), "Expected valid data");
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test --all-features

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test integration_test

# Run specific test
cargo test test_validation_with_real_schema

# Run with output
cargo test -- --nocapture

# Generate coverage report
cargo tarpaulin --all-features --out Html
```

### Test Organization

- Unit tests: In the same file as the code, in a `tests` module
- Integration tests: In `tests/` directory
- Test utilities: In a separate `test-utils` crate or module

## Documentation

### Code Documentation

All public APIs must be documented:

```rust
/// Validates data against a LinkML schema.
///
/// # Arguments
///
/// * `schema` - The LinkML schema to validate against
/// * `data` - The data to validate (as JSON value)
/// * `target_class` - The name of the class to validate
///
/// # Returns
///
/// Returns a `ValidationResult` containing validation status and any errors.
///
/// # Errors
///
/// Returns `LinkMLError` if:
/// - Schema is invalid
/// - Target class not found
/// - Data format is incorrect
///
/// # Example
///
/// ```rust
/// use linkml_service::{create_linkml_service, LinkMLService};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let service = create_linkml_service().await?;
///     let schema = service.load_schema("schema.yaml").await?;
///     let result = service.validate_data(&schema, &data, "Person").await?;
///     Ok(())
/// }
/// ```
pub async fn validate_data(
    &self,
    schema: &Schema,
    data: &serde_json::Value,
    target_class: &str,
) -> Result<ValidationResult, LinkMLError> {
    // Implementation
}
```

### Updating Documentation

When adding features or changing APIs:

1. Update relevant markdown files in `docs/`
2. Update `CHANGELOG.md`
3. Add examples if applicable
4. Update README.md if needed

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass**:
   ```bash
   cargo test --all-features
   ```

2. **Run code quality checks**:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features
   ```

3. **Update documentation**:
   - Add/update code comments
   - Update relevant markdown docs
   - Add CHANGELOG entry

4. **Verify examples still work**:
   ```bash
   cargo run --example basic_usage
   ```

### Submitting a Pull Request

1. Push your changes to your fork
2. Create a Pull Request on GitHub
3. Fill out the PR template completely
4. Link any related issues
5. Wait for CI checks to pass
6. Respond to review feedback

### PR Template

```markdown
## Description

Brief description of the changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issues

Fixes #123

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests passing
- [ ] Manual testing performed

## Checklist

- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings introduced
- [ ] CHANGELOG.md updated
```

### Review Process

- Maintainers will review your PR within 7 days
- Address review feedback promptly
- Keep discussions focused and professional
- Be open to suggestions and alternative approaches

## Release Process

Releases are managed by project maintainers following semantic versioning:

1. Version bump in all `Cargo.toml` files
2. Update `CHANGELOG.md` with release notes
3. Create git tag: `git tag v2.1.0`
4. Push tag: `git push origin v2.1.0`
5. GitHub Actions will automatically:
   - Run full test suite
   - Build documentation
   - Publish to crates.io (if configured)
   - Create GitHub release

## Getting Help

- **Documentation**: Check the [docs/](docs/) directory
- **Issues**: Search existing issues or create a new one
- **Discussions**: Use GitHub Discussions for questions
- **Contact**: Simon C. Kemper <textpast@textpast.com>

## Recognition

Contributors will be recognized in:
- `CHANGELOG.md` for significant contributions
- GitHub contributors page
- Release notes

Thank you for contributing to LinkML Rust! 🦀
