# Unit Testing Plan for Thwarter IF Engine

## Overview

This document outlines a comprehensive unit testing strategy for the Thwarter interactive fiction engine. The plan covers all core modules and ensures robust testing coverage for the P0 implementation.

## Testing Strategy

### Test Organization
- **Unit Tests**: Test individual functions and structs in isolation
- **Integration Tests**: Test module interactions and data flow
- **Property Tests**: Use fuzzing for parser and validation logic
- **Mock Dependencies**: Use test doubles for file I/O and external dependencies

### Coverage Goals
- **Models**: 95% coverage - Core data structures must be bulletproof
- **Parsers**: 90% coverage - Critical for data integrity
- **Engines**: 85% coverage - Complex business logic requires thorough testing
- **CLI**: 80% coverage - User interface logic with acceptable test complexity

## Module-by-Module Testing Plan

### 1. Models Module (`src/models/`)

#### 1.1 World Models (`src/models/world/`)

**`location.rs`**
- `Location` struct creation and validation
- YAML frontmatter parsing edge cases
- Exit validation and bidirectional checking
- Object and NPC placement logic
- Coordinate system integration

**`coordinate.rs`**
- Coordinate arithmetic and validation
- Distance calculations
- Boundary checking
- Invalid coordinate handling

**`world_graph.rs`**
- Graph construction from locations
- Path finding algorithms
- Cycle detection
- Orphaned location identification
- Graph traversal methods

#### 1.2 Character Models (`src/models/character/`)

**`character.rs`**
- Character creation and attribute validation
- Dialogue system testing
- Character state transitions
- Inventory management
- Skill/attribute calculations

**`player.rs`**
- Player-specific functionality
- Character progression systems
- Player state persistence
- Action validation

**`character_state.rs`**
- State machine transitions
- State validation and constraints
- State serialization/deserialization

#### 1.3 Narrative Models (`src/models/narrative/`)

**`story_variable.rs`**
- Variable creation and type checking
- Variable scope and persistence
- Expression evaluation
- Variable interpolation in text

**`game_state.rs`**
- Game state initialization
- State mutation and rollback
- State validation and consistency
- Save/load functionality

**`narrative_arc.rs`**
- Arc progression logic
- Trigger condition evaluation
- Arc completion detection
- Arc dependency handling

**`plot_point.rs`**
- Plot point activation logic
- Condition checking
- State change effects
- Plot point chaining

#### 1.4 Game Models (`src/models/game/`)

**`action.rs`**
- Action parsing and validation
- Action effect application
- Action precondition checking
- Action history tracking

**`game.rs`**
- Game initialization
- Turn processing
- Game rule enforcement
- Win/lose condition evaluation

**`game_session.rs`**
- Session lifecycle management
- Session state persistence
- Multi-session handling
- Session cleanup

### 2. Parsers Module (`src/parsers/`)

#### 2.1 Markdown Parser (`markdown_parser.rs`)
- CommonMark compliance testing
- Frontmatter extraction
- Content body parsing
- Error handling for malformed markdown
- Custom IF syntax parsing
- Performance testing with large files

#### 2.2 Metadata Parser (`metadata_parser.rs`)
- YAML parsing edge cases
- JSON parsing support
- TOML configuration parsing
- Schema validation
- Error reporting and recovery
- Type coercion and validation

### 3. Engines Module (`src/engines/`)

#### 3.1 World Engine (`world_engine.rs`)
- World loading and validation
- Location graph construction
- Reference resolution
- Circular dependency detection
- Performance with large worlds
- Memory management

#### 3.2 Character Engine (`character_engine.rs`)
- Character lifecycle management
- NPC behavior simulation
- Character interaction logic
- Dialogue system processing
- Character state synchronization

#### 3.3 Narrative Engine (`narrative_engine.rs`)
- Story progression logic
- Variable tracking and updates
- Conditional content evaluation
- Arc and plot point management
- Narrative state consistency

#### 3.4 Game Engine (`game_engine.rs`)
- Game loop execution
- Action processing pipeline
- Rule enforcement
- Event handling and propagation
- Game state management

### 4. CLI Module (`src/cli/`)

#### 4.1 Commands (`src/cli/commands/`)

**`validate.rs`**
- World validation logic
- Error reporting accuracy
- Performance with large datasets
- Reference checking
- Configuration validation

**`show.rs`**
- State display formatting
- Output format selection
- Filter and search integration
- Performance optimization

**`map.rs`**
- ASCII map generation
- Coordinate mapping accuracy
- Fog-of-war implementation
- Scale and boundary handling
- Character/object visualization

**`search.rs`**
- Content search algorithms
- Regex pattern matching
- Cross-reference finding
- Search result ranking
- Performance optimization

**`debug.rs`**
- Diagnostic information gathering
- Debug output formatting
- State inspection tools
- Performance metrics

#### 4.2 Configuration (`config.rs`)
- Configuration loading and validation
- Default value handling
- Environment variable integration
- Configuration file precedence
- Error handling and recovery

#### 4.3 Output Formatting (`output.rs`)
- JSON output formatting
- YAML output formatting
- Text output formatting
- Colored output handling
- Format validation

## Test Implementation Strategy

### Phase 1: Core Models (Week 1-2)
1. Set up test infrastructure and utilities
2. Implement tests for all model structs
3. Create test fixtures and sample data
4. Establish testing patterns and conventions

### Phase 2: Parsers and Validation (Week 3)
1. Parser unit tests with edge cases
2. Error handling and recovery testing
3. Performance testing with large files
4. Fuzz testing for parser robustness

### Phase 3: Engines (Week 4-5)
1. Engine integration testing
2. Mock external dependencies
3. State management testing
4. Performance and memory testing

### Phase 4: CLI and Integration (Week 6)
1. CLI command testing
2. Output format validation
3. End-to-end integration tests
4. Error handling and user experience

## Test Infrastructure

### Test Utilities
```rust
// src/test_utils.rs
mod test_utils {
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    pub fn create_test_world() -> (TempDir, PathBuf) { /* ... */ }
    pub fn create_test_location(name: &str) -> Location { /* ... */ }
    pub fn create_test_character(name: &str) -> Character { /* ... */ }
    pub fn assert_valid_world(world: &World) { /* ... */ }
}
```

### Test Data Management
- **Fixtures**: Standardized test data in `tests/fixtures/`
- **Generators**: Property-based test data generation
- **Cleanup**: Automatic cleanup of temporary test files
- **Isolation**: Each test runs in isolated environment

### Mocking Strategy
- **File System**: Mock file operations for isolated testing
- **Network**: Mock any future network dependencies
- **Time**: Mock time-dependent functionality
- **Random**: Seed random number generation for reproducible tests

## Test Categories

### Unit Tests
```bash
# Run all unit tests
cargo test --lib

# Run specific module tests
cargo test models::world::location
cargo test engines::world_engine
cargo test parsers::markdown_parser
```

### Integration Tests
```bash
# Run integration tests
cargo test --test integration

# Run CLI integration tests
cargo test --test cli_integration
```

### Property Tests
```bash
# Run property-based tests
cargo test --test property_tests
```

### Performance Tests
```bash
# Run benchmark tests
cargo test --release --test benchmarks
```

## Test Coverage and Quality

### Coverage Tools
- Use `cargo-tarpaulin` for coverage reporting
- Set up CI/CD coverage reporting
- Maintain minimum coverage thresholds
- Track coverage trends over time

### Code Quality
- Use `cargo-audit` for security scanning
- Use `cargo-clippy` for linting
- Use `cargo-fmt` for formatting
- Use `cargo-deny` for dependency validation

### Test Quality Metrics
- Test execution time monitoring
- Test flakiness detection
- Test maintenance burden tracking
- Documentation coverage for public APIs

## Continuous Integration

### CI Pipeline
```yaml
# .github/workflows/tests.yml
- name: Unit Tests
  run: cargo test --lib --verbose
  
- name: Integration Tests  
  run: cargo test --test integration --verbose
  
- name: Property Tests
  run: cargo test --test property_tests --verbose
  
- name: Coverage Report
  run: cargo tarpaulin --out xml --output-dir coverage/
```

### Quality Gates
- All tests must pass
- Coverage must meet minimum thresholds
- No security vulnerabilities
- No clippy warnings
- Formatted code only

## Documentation Testing

### Doc Tests
- Ensure all public API examples work
- Test code examples in documentation
- Validate README examples
- Test CLI help examples

### Example Testing
- Validate sample world content
- Test all tutorial examples
- Verify getting started guides
- Test configuration examples

## Success Metrics

### Quantitative Goals
- **Test Count**: 200+ unit tests across all modules
- **Coverage**: 85%+ overall, 95%+ for models
- **Performance**: All tests complete under 30 seconds
- **Reliability**: 99.9% test stability (no flaky tests)

### Qualitative Goals
- **Maintainability**: Tests are easy to understand and modify
- **Documentation**: All public APIs have working examples
- **Confidence**: Team confidence in refactoring and changes
- **Quality**: Bugs caught in development, not production

## Implementation Timeline

- **Week 1**: Models module testing (40 tests)
- **Week 2**: Models completion + Parser testing (80 tests)
- **Week 3**: Engine testing (120 tests)
- **Week 4**: CLI testing (160 tests)
- **Week 5**: Integration and performance testing (200 tests)
- **Week 6**: Documentation, cleanup, and optimization

## Risk Mitigation

### Test Maintenance
- Keep tests simple and focused
- Use shared test utilities
- Regular test review and cleanup
- Automated test dependency updates

### Performance Risks
- Monitor test execution time
- Parallelize test execution
- Use test sharding for large suites
- Profile and optimize slow tests

### Coverage Gaps
- Regular coverage analysis
- Focus on critical path testing
- Property testing for edge cases
- Manual testing for UI/UX elements

## Next Steps

1. **Review and Approval**: Present plan to team for feedback
2. **Infrastructure Setup**: Create test utilities and CI pipeline
3. **Phase 1 Implementation**: Begin with core models testing
4. **Iterative Development**: Implement tests alongside features
5. **Continuous Improvement**: Regular test suite optimization

This plan provides comprehensive coverage for the Thwarter IF engine while maintaining focus on the most critical components and ensuring long-term maintainability.