# Code Style and Conventions

## Formatting Configuration

**rustfmt.toml settings**:
- `edition = "2021"`
- `max_width = 100` - Maximum line width
- `chain_width = 60` - Chain formatting width
- `match_block_trailing_comma = true` - Trailing commas in match arms
- `use_try_shorthand = true` - Use `?` operator
- `use_field_init_shorthand = true` - Use field init shorthand
- `force_explicit_abi = true` - Explicit ABI in extern declarations

**Important**: Only stable rustfmt features are used. No unstable features.

## Code Quality Requirements

1. **Formatting**: Must pass `cargo fmt --all -- --check`
2. **Clippy**: Must pass `cargo clippy --all-targets --all-features -- -D warnings`
   - All warnings treated as errors in CI
3. **Tests**: All tests must pass
4. **Documentation**: Doc comments for public APIs

## Naming Conventions

Following standard Rust conventions:
- `snake_case` for functions, variables, modules
- `CamelCase` for types, structs, enums
- `SCREAMING_SNAKE_CASE` for constants
- Clear, descriptive names preferred over abbreviations

## Project-Specific Patterns

1. **Error Handling**: 
   - Use `Result<T>` type alias from `error.rs`
   - Use `IntentError` for domain errors
   - Use `thiserror` for error definitions

2. **Async**: 
   - All database operations are async
   - Use tokio runtime
   - Entry point: `#[tokio::main]`

3. **Database**:
   - SQLx for type-safe SQL queries
   - Use prepared statements (bind parameters)
   - Transaction support where needed

4. **CLI Design**:
   - Declarative with clap derive
   - Subcommands for logical grouping
   - JSON output for machine consumption
