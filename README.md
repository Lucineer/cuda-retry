# cuda-retry

Retry — exponential backoff, jitter, circuit breaker, resilient execution patterns (Rust)

Part of the Cocapn fleet — a Lucineer vessel component.

## What It Does

### Key Types

- `RetryPolicy` — core data structure
- `CircuitBreaker` — core data structure
- `Attempt` — core data structure
- `RetryTracker` — core data structure

## Quick Start

```bash
# Clone
git clone https://github.com/Lucineer/cuda-retry.git
cd cuda-retry

# Build
cargo build

# Run tests
cargo test
```

## Usage

```rust
use cuda_retry::*;

// See src/lib.rs for full API
// 12 unit tests included
```

### Available Implementations

- `Default for RetryPolicy` — see source for methods
- `RetryPolicy` — see source for methods
- `CircuitBreaker` — see source for methods
- `RetryTracker` — see source for methods

## Testing

```bash
cargo test
```

12 unit tests covering core functionality.

## Architecture

This crate is part of the **Cocapn Fleet** — a git-native multi-agent ecosystem.

- **Category**: other
- **Language**: Rust
- **Dependencies**: See `Cargo.toml`
- **Status**: Active development

## Related Crates


## Fleet Position

```
Casey (Captain)
├── JetsonClaw1 (Lucineer realm — hardware, low-level systems, fleet infrastructure)
├── Oracle1 (SuperInstance — lighthouse, architecture, consensus)
└── Babel (SuperInstance — multilingual scout)
```

## Contributing

This is a fleet vessel component. Fork it, improve it, push a bottle to `message-in-a-bottle/for-jetsonclaw1/`.

## License

MIT

---

*Built by JetsonClaw1 — part of the Cocapn fleet*
*See [cocapn-fleet-readme](https://github.com/Lucineer/cocapn-fleet-readme) for the full fleet roadmap*
