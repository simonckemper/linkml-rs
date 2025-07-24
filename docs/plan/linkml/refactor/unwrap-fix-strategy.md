# Unwrap() Fix Strategy for LinkML Service

## Overview

This document outlines the strategy for systematically fixing ~2400 unwrap() instances in the LinkML service modules to comply with RootReal's zero-tolerance policy.

## Current State

- **Total unwrap() instances**: ~2400 (excluding test code)
- **Critical modules identified**:
  - Plugin system (registry, compatibility, discovery)
  - Generators (30+ generator implementations)
  - Parsers (YAML, JSON, import resolvers)
  - Validators (compiled, engine, validators)
  - Expression engine
  - CLI modules

## Categorization of unwrap() Usage

### 1. Safe Patterns (Lower Priority)
These patterns are generally safe but should still be replaced:

- **Progress bar templates**: `.template("{...}").unwrap()`
  - Replace with: `.template("{...}").expect("valid template")`
  
- **Serialization of known types**: `serde_json::to_string(&known_type).unwrap()`
  - Replace with: `serde_json::to_string(&known_type).expect("serialization should not fail")`
  
- **Regex compilation with literals**: `Regex::new(r"literal").unwrap()`
  - Replace with: `lazy_static!` or `once_cell` for compile-time validation

### 2. Dangerous Patterns (High Priority)
These must be fixed immediately:

- **After is_some() check**: `if opt.is_some() { opt.unwrap() }`
  - Replace with: `if let Some(val) = opt { ... }`
  
- **Collection access**: `vec[index].unwrap()`, `map.get(key).unwrap()`
  - Replace with: Proper bounds checking and error handling
  
- **Parse operations**: `str.parse().unwrap()`
  - Replace with: Return Result or provide context

- **File operations**: `fs::read(path).unwrap()`
  - Replace with: Proper error propagation

### 3. Test-like Code in Production
Some production code uses unwrap() in ways that suggest it was written for testing:

- **Temporary directory creation**: `TempDir::new().unwrap()`
- **Example/demo code**: Should use proper error handling

## Fix Strategy by Module

### Phase 1: Critical Path Modules (High Priority)

#### 1.1 Plugin System (`plugin/*.rs`)
- **Files**: registry.rs, compatibility.rs, discovery.rs
- **Pattern**: Many `RwLock` unwraps that should use proper locking
- **Fix**: 
  ```rust
  // Instead of: self.plugins.read().unwrap()
  self.plugins.read()
      .map_err(|_| PluginError::LockPoisoned("plugins"))?
  ```

#### 1.2 Validators (`validator/*.rs`)
- **Critical**: engine.rs, compiled.rs, validators/*.rs
- **Pattern**: Cache access, JSON number conversion
- **Fix**: Proper error types and context

#### 1.3 Parsers (`parser/*.rs`)
- **Files**: yaml_parser.rs, json_parser.rs, import_resolver.rs
- **Pattern**: File I/O and parsing operations
- **Fix**: Propagate parse errors with context

### Phase 2: Generator Modules (Medium Priority)

#### 2.1 Code Generators (`generator/*.rs`)
- **Count**: 30+ generator files
- **Pattern**: String formatting, file writing
- **Fix**: Use `write!` macro with error handling

#### 2.2 TypeQL Generators
- **Special attention**: Complex logic with many unwraps
- **Fix**: Create generator-specific error types

### Phase 3: Expression Engine (`expression/*.rs`)
- **Pattern**: Numeric operations, function calls
- **Fix**: Handle overflow/underflow, invalid operations

### Phase 4: CLI and Utilities
- **Pattern**: User input handling, file operations
- **Fix**: User-friendly error messages

## Implementation Approach

### 1. Automated Fixes (Where Possible)
Create a script to handle safe patterns:
```bash
# Replace safe patterns with expect()
rg "\.template\(.*?\)\.unwrap\(\)" -r '.template($1).expect("valid template")'
```

### 2. Manual Review Required
For each dangerous pattern:
1. Understand the context
2. Determine appropriate error handling
3. Add proper error types if needed
4. Test the fix

### 3. Module-by-Module Approach
1. Start with one module
2. Fix all unwraps in that module
3. Run tests to ensure no regressions
4. Document any behavior changes
5. Move to next module

## Error Handling Guidelines

### 1. Use Appropriate Error Types
- Don't use `Box<dyn Error>` everywhere
- Create specific error types for each module
- Use `thiserror` for error derivation

### 2. Provide Context
```rust
// Bad
let val = str.parse().unwrap();

// Good
let val = str.parse()
    .map_err(|e| ParseError::InvalidFormat {
        input: str.to_string(),
        expected: "valid integer",
        error: e.to_string(),
    })?;
```

### 3. Consider Recovery
- Some operations can have sensible defaults
- Others must propagate errors
- Document the decision

## Progress Tracking

### Modules to Fix:
- [x] plugin/registry.rs (22 unwraps fixed - only test code remains)
- [x] plugin/compatibility.rs (3 unwraps fixed - only test code remains)
- [x] plugin/discovery.rs (3 unwraps fixed - test code converted to Result<()>)
- [x] validator/engine.rs (3 unwraps fixed)
- [x] validator/compiled.rs (4 unwraps fixed - 3 from_f64, 1 test expect)
- [x] parser/yaml_parser.rs (2 unwraps fixed - test code using expect)
- [x] parser/json_parser.rs (1 unwrap fixed - test code using expect)
- [x] generator/typeql_generator.rs (37 unwraps fixed - all write!/writeln! with error mapping)
- [x] generator/python_dataclass.rs (53 unwraps fixed - all write!/writeln! with error mapping)
- [ ] expression/evaluator.rs (20+ unwraps)
- [ ] (... and many more)

### Metrics:
- Starting unwraps: ~2400
- Fixed: ~128 (production code + test code)
- Remaining: ~2272
- Target: 0

### Progress Log:
- **2025-01-31**: Started with plugin/registry.rs as pilot
  - Fixed all RwLock unwrap() calls with proper error handling
  - Replaced LinkMLError::PluginError with LinkMLError::ServiceError
  - Pattern established: RwLock errors map to "lock poisoned" messages
- **2025-01-31**: Fixed plugin/compatibility.rs
  - Fixed env! macro unwrap() calls with expect()
  - Test unwrap() calls left unchanged (acceptable)
- **2025-01-31**: Fixed plugin/discovery.rs
  - Fixed test unwrap() calls by converting to Result<()>
  - Used LinkMLError::IOError for file operations
  - Pattern established: Test functions can return Result<()> for proper error handling
- **2025-01-31**: Fixed validator/compiled.rs
  - Fixed serde_json::Number::from_f64() unwrap() calls with if-let pattern
  - Pattern established: Use if-let for optional conversions that don't need to fail validation
  - Test unwrap() converted to expect() with descriptive message
- **2025-01-31**: Fixed parser modules (yaml_parser.rs and json_parser.rs)
  - Fixed test unwrap() calls with expect() and descriptive messages
  - All parser modules now unwrap-free
- **2025-01-31**: Fixed generator/typeql_generator.rs
  - Fixed 37 unwrap() calls from write!/writeln! macros
  - Added fmt_error_to_generator_error helper function
  - Pattern established: Convert fmt::Error to GeneratorError::Io for string formatting operations
  - Used map_err with ? operator for proper error propagation
- **2025-01-31**: Fixed generator/python_dataclass.rs
  - Fixed 53 unwrap() calls from write!/writeln! macros
  - Used same fmt_error_to_generator_error pattern as typeql_generator
  - Note: unwrap_or() is safe and doesn't need replacement (provides default value)

## Testing Strategy

1. **Unit Tests**: Ensure error paths are tested
2. **Integration Tests**: Verify error propagation
3. **Regression Tests**: No behavior changes for success paths
4. **Error Message Quality**: User-friendly messages

## Timeline Estimate

Given ~2400 unwraps and complexity:
- Automated fixes: 1 day (~500 unwraps)
- Manual fixes: 10-15 unwraps/hour
- Total estimate: 2-3 weeks of focused effort

## Next Steps

1. Start with plugin/registry.rs as a pilot
2. Develop patterns and helpers
3. Apply learnings to other modules
4. Create PR for each module group
5. Update this document with progress