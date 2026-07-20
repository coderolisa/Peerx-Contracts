# PeerX CLI

Command-line interface for managing and monitoring PeerX smart contracts on Soroban.

## Features

- **Health Checks**: Pre-flight validation of PeerX contract infrastructure
- **Structured Output**: Support for human-readable, JSON, and YAML formats
- **Exit Codes**: Standard exit codes (0=healthy, 1=warnings, 2=critical)
- **Flexible Configuration**: Environment variables, config files, and CLI arguments

## Installation

### From Source

```bash
cd peerx-cli
cargo build --release
```

The binary will be available at `target/release/peerx`.

### Add to PATH

```bash
# Linux/macOS
export PATH="$PATH:/path/to/peerx-cli/target/release"

# Or install globally
cargo install --path .
```

## Quick Start

### Basic Health Check

```bash
# Set required environment variables
export PEERX_CONTRACT_ID="CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
export PEERX_NETWORK="testnet"

# Run health check
peerx health
```

### With All Options

```bash
peerx health \
  --network testnet \
  --contract-id CXXXXX... \
  --rpc-url https://soroban-testnet.stellar.org \
  --admin-address GXXXXX... \
  --format json \
  --details
```

## Commands

### `peerx health`

Run pre-flight health checks on PeerX contract and infrastructure.

**Checks Performed:**

1. **RPC Reachable** (Critical) - Verifies Soroban RPC endpoint is accessible
2. **Horizon Reachable** (Warning) - Verifies Horizon API is accessible
3. **Admin Reachable** (Warning/Fail) - Verifies admin account exists on-chain
4. **Contract Exists** (Critical) - Validates contract ID format and existence
5. **Contract Not Paused** (Critical) - Checks if contract operations are paused
6. **Oracle Fresh** (Warning) - Verifies oracle data is within freshness threshold

**Exit Codes:**

- `0` - All checks passed (healthy)
- `1` - Some warnings present (degraded)
- `2` - Critical failures (unhealthy)

**Options:**

```
-n, --network <NETWORK>           Network to check [env: PEERX_NETWORK]
-c, --contract-id <CONTRACT_ID>   Contract ID to check [env: PEERX_CONTRACT_ID]
-r, --rpc-url <RPC_URL>           RPC endpoint URL [env: PEERX_RPC_URL]
-a, --admin-address <ADDRESS>     Admin address to verify [env: PEERX_ADMIN_ADDRESS]
-d, --details                     Show detailed check information
    --only <CHECKS>               Only run specific checks (comma-separated)
-f, --format <FORMAT>             Output format: json, yaml, or human [default: human]
-v, --verbose                     Enable verbose output
-q, --quiet                       Suppress all output except errors
-h, --help                        Print help
```

**Examples:**

```bash
# Basic health check
peerx health

# With specific network
peerx health --network mainnet

# JSON output for automation
peerx health --format json

# Show detailed check information
peerx health --details

# Run specific checks only
peerx health --only rpc_reachable,contract_not_paused

# Quiet mode for scripts
peerx health --quiet
echo $?  # Check exit code
```

## Configuration

### Environment Variables

```bash
# Required
export PEERX_CONTRACT_ID="CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

# Optional (with defaults)
export PEERX_NETWORK="testnet"
export PEERX_RPC_URL="https://soroban-testnet.stellar.org"
export PEERX_ADMIN_ADDRESS="GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
```

### Configuration File

Create `peerx-cli.json` in your project directory or `~/.config/peerx/config.json`:

```json
{
  "network": {
    "name": "testnet",
    "rpc_url": "https://soroban-testnet.stellar.org",
    "horizon_url": "https://horizon-testnet.stellar.org",
    "passphrase": "Test SDF Network ; September 2015"
  },
  "contract": {
    "contract_id": "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
    "admin_address": "GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
  },
  "health": {
    "timeout_seconds": 30,
    "oracle_freshness_threshold_seconds": 300,
    "max_retries": 3
  }
}
```

### Configuration Precedence

Configuration is loaded in the following order (later sources override earlier):

1. Default values
2. Configuration file (`peerx-cli.json` or `~/.config/peerx/config.json`)
3. Environment variables
4. Command-line arguments

## Usage in Scripts

### Bash Script

```bash
#!/bin/bash

# Run health check
peerx health --format json --quiet > health-report.json
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo "✓ System healthy"
    exit 0
elif [ $EXIT_CODE -eq 1 ]; then
    echo "⚠ System degraded"
    exit 1
else
    echo "✗ System unhealthy"
    exit 2
fi
```

### CI/CD Integration

#### GitHub Actions

```yaml
name: Health Check
on:
  schedule:
    - cron: '*/5 * * * *'  # Every 5 minutes

jobs:
  health-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build CLI
        run: |
          cd peerx-cli
          cargo build --release
      
      - name: Run Health Check
        env:
          PEERX_CONTRACT_ID: ${{ secrets.CONTRACT_ID }}
          PEERX_NETWORK: testnet
        run: |
          ./peerx-cli/target/release/peerx health --format json
```

#### GitLab CI

```yaml
health-check:
  stage: monitor
  script:
    - cd peerx-cli
    - cargo build --release
    - ./target/release/peerx health --format json
  only:
    - schedules
```

### Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: peerx-health-check
spec:
  schedule: "*/5 * * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: health-check
            image: peerx-cli:latest
            env:
            - name: PEERX_CONTRACT_ID
              valueFrom:
                secretKeyRef:
                  name: peerx-config
                  key: contract-id
            command:
            - /usr/local/bin/peerx
            - health
            - --format
            - json
          restartPolicy: OnFailure
```

## Output Formats

### Human-Readable (default)

```
PeerX Health Check
==================================================
Network: testnet
Contract: CXXXXX...

Check Results:
--------------------------------------------------
✓ rpc_reachable................... (45ms)
✓ horizon_reachable............... (67ms)
✓ admin_reachable................. (89ms)
✓ contract_exists................. (12ms)
✓ contract_not_paused............. (34ms)
⚠ oracle_fresh.................... (23ms)
  Oracle data may be stale (updated 320 seconds ago, threshold: 300s)

Summary:
--------------------------------------------------
  Total checks: 6
  ✓ Passed: 5
  ⚠ Warnings: 1

--------------------------------------------------
⚠ Some checks failed - System is degraded

ℹ Completed in 270ms
ℹ Exit code: 1
```

### JSON

```json
{
  "status": "degraded",
  "timestamp": "2024-01-15T10:30:45.123Z",
  "checks": [
    {
      "name": "rpc_reachable",
      "status": "pass",
      "message": "RPC endpoint is reachable at https://soroban-testnet.stellar.org",
      "duration_ms": 45,
      "details": {
        "url": "https://soroban-testnet.stellar.org",
        "status_code": 200,
        "response_time_ms": 45
      }
    },
    {
      "name": "oracle_fresh",
      "status": "warn",
      "message": "Oracle data may be stale (updated 320 seconds ago, threshold: 300s)",
      "duration_ms": 23,
      "details": {
        "last_update_seconds_ago": 320,
        "threshold_seconds": 300,
        "is_fresh": false
      }
    }
  ],
  "total_duration_ms": 270,
  "summary": {
    "total": 6,
    "passed": 5,
    "warnings": 1,
    "failed": 0,
    "errors": 0
  }
}
```

## Troubleshooting

### Common Issues

**"Contract ID is required"**

Set the contract ID via environment variable or config file:
```bash
export PEERX_CONTRACT_ID="CXXXXX..."
```

**"RPC endpoint unreachable"**

Check your network connection and RPC URL:
```bash
curl https://soroban-testnet.stellar.org/health
```

**"Invalid contract ID format"**

Ensure the contract ID is 56 characters and starts with 'C':
```bash
peerx health --contract-id CXXXXX...
```

**Timeout errors**

Increase timeout in config:
```json
{
  "health": {
    "timeout_seconds": 60
  }
}
```

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Output

```bash
RUST_LOG=debug peerx health --verbose
```

### Building for Production

```bash
cargo build --release --locked
strip target/release/peerx  # Optional: reduce binary size
```

## Contributing

Contributions welcome! Please ensure:

1. All tests pass: `cargo test`
2. Code is formatted: `cargo fmt`
3. No clippy warnings: `cargo clippy`
4. Documentation is updated

## License

MIT License - See LICENSE file for details

## Support

- **Issues**: https://github.com/coderolisa/Peerx-Contracts/issues
- **Documentation**: https://github.com/coderolisa/Peerx-Contracts/tree/main/peerx-cli
