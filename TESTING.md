# Testing Guide

This document provides comprehensive testing examples and strategies for Thwarter interactive fiction worlds.

## Unit Testing with Cargo

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test models::

# Run tests in release mode
cargo test --release
```

## World Validation Testing

### Basic Validation

```bash
# Validate entire world
thwart validate authors/world/

# Validate specific location
thwart validate authors/world/castle/courtyard.md

# Validate with verbose output
thwart validate authors/world/ --verbose

# Check for broken references
thwart validate authors/world/ --check-refs
```

### Advanced Validation Scenarios

```bash
# Test with missing files
thwart validate authors/world/ --strict

# Validate character consistency
thwart validate authors/world/npcs/

# Test world configuration
thwart validate authors/world/world.toml
```

## Interactive Testing

### State Testing

```bash
# Test initial world state
thwart show authors/world/ --format json

# Test specific location state
thwart show authors/world/castle/courtyard.md --state initial

# Test with character interactions
thwart show authors/world/castle/courtyard.md --with-npcs

# Test object placement
thwart show authors/world/castle/courtyard.md --with-objects
```

### Navigation Testing

```bash
# Test all exits from a location
thwart debug authors/world/castle/courtyard.md --check-exits

# Test circular navigation
thwart debug authors/world/ --check-cycles

# Test unreachable locations
thwart debug authors/world/ --check-orphans
```

## Map Generation Testing

### ASCII Map Testing

```bash
# Generate full world map
thwart map authors/world/

# Test with fog of war
thwart map authors/world/ --fow

# Test specific area mapping
thwart map authors/world/castle/

# Test map with character positions
thwart map authors/world/ --show-npcs

# Test map scaling
thwart map authors/world/ --scale 2
```

### Map Validation

```bash
# Verify map connectivity
thwart map authors/world/ --validate-paths

# Check map boundaries
thwart map authors/world/ --check-bounds

# Test map rendering formats
thwart map authors/world/ --format svg
```

## Search and Content Testing

### Content Discovery

```bash
# Search for specific terms
thwart search "treasure" authors/world/

# Search character dialogue
thwart search "king" authors/world/npcs/

# Search with regex patterns
thwart search "sword|blade" authors/world/ --regex

# Search metadata only
thwart search "exits" authors/world/ --metadata-only
```

### Cross-Reference Testing

```bash
# Find all references to a character
thwart search "guard-captain" authors/world/ --cross-ref

# Find unused objects
thwart search --unused-objects authors/world/

# Find missing descriptions
thwart search --missing-descriptions authors/world/
```

## Performance Testing

### Load Testing

```bash
# Time world loading
time thwart show authors/world/

# Memory usage profiling
valgrind thwart show authors/world/

# Large world testing
thwart validate large-world/ --benchmark
```

### Stress Testing

```bash
# Test with many concurrent operations
for i in {1..100}; do thwart validate authors/world/ & done; wait

# Test with large files
thwart validate stress-test/huge-location.md

# Test memory limits
thwart show stress-test/ --max-memory 100MB
```

## Integration Testing Scenarios

### Story Flow Testing

Create test scenarios to validate story progression:

```bash
# Test Act 1 progression
thwart debug authors/world/ --scenario act1-start
thwart validate authors/world/castle/ --story-flow

# Test character arc completion
thwart debug authors/world/npcs/king.md --arc-complete

# Test alternate endings
thwart validate authors/world/ --all-endings
```

### Multi-Location Testing

```bash
# Test location transitions
thwart debug authors/world/ --trace-movements

# Test inventory persistence
thwart show authors/world/castle/courtyard.md --carry-items

# Test state changes across locations
thwart debug authors/world/ --state-tracking
```

## Error Testing

### Intentional Error Scenarios

```bash
# Test with malformed YAML
thwart validate test-data/broken-frontmatter.md

# Test with circular references
thwart validate test-data/circular-exits/

# Test with missing dependencies
thwart validate test-data/missing-npcs.md

# Test with invalid markdown
thwart validate test-data/bad-markdown.md
```

### Error Recovery Testing

```bash
# Test graceful failure modes
thwart show test-data/corrupted-world/ --safe-mode

# Test partial loading
thwart validate test-data/incomplete-world/ --partial

# Test error reporting
thwart validate test-data/multiple-errors/ --report-all
```

## Automated Testing Scripts

### Test Suite Runner

```bash
#!/bin/bash
# run-tests.sh

echo "Running validation tests..."
thwart validate authors/world/ || exit 1

echo "Running map generation tests..."
thwart map authors/world/ --validate || exit 1

echo "Running search tests..."
thwart search ".*" authors/world/ --regex --count || exit 1

echo "Running performance tests..."
time thwart show authors/world/ > /dev/null || exit 1

echo "All tests passed!"
```

### Continuous Integration

```yaml
# .github/workflows/test.yml
name: Test Thwarter Worlds

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    - name: Build
      run: cargo build --release
    - name: Run unit tests
      run: cargo test
    - name: Validate world
      run: ./target/release/thwart validate authors/world/
    - name: Test map generation
      run: ./target/release/thwart map authors/world/
    - name: Run integration tests
      run: ./scripts/run-tests.sh
```

## Test Data Management

### Creating Test Worlds

```bash
# Minimal test world
mkdir -p test-worlds/minimal/{castle,npcs}
echo "title: Test Room" > test-worlds/minimal/castle/room.md
echo "title: Test NPC" > test-worlds/minimal/npcs/test.md

# Complex test scenarios
cp -r authors/world/ test-worlds/complex/
# Modify files to create specific test conditions
```

### Test Data Cleanup

```bash
# Clean generated test files
find test-outputs/ -name "*.json" -delete
find test-outputs/ -name "*.yaml" -delete

# Reset test world state
git checkout test-worlds/
```

## Debugging and Diagnostics

### Debug Mode Testing

```bash
# Enable debug logging
RUST_LOG=debug thwart validate authors/world/

# Trace execution
RUST_LOG=trace thwart show authors/world/castle/courtyard.md

# Memory debugging
RUST_LOG=debug thwart debug authors/world/ --memory-stats
```

### Profile-Guided Testing

```bash
# Profile world loading
perf record thwart show authors/world/
perf report

# Profile map generation
flamegraph thwart map authors/world/

# Profile validation
hyperfine 'thwart validate authors/world/'
```