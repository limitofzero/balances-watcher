# Token Balances Updater

Real-time ERC20 token balance tracking service with SSE (Server-Sent Events) support.

## Features

- Real-time balance updates via SSE
- Multicall for batch balance queries
- WebSocket subscriptions for Transfer events
- Multi-chain support (Ethereum, Arbitrum, Sepolia)

## API Endpoints

### Get Token List

```bash
curl http://localhost:8080/{chain_id}/tokens-list
```

**Example:**
```bash
curl http://localhost:8080/1/tokens-list
```

### Get Single Token Balance

```bash
curl http://localhost:8080/{chain_id}/balance/{owner}/{token}
```

**Example:**
```bash
curl http://localhost:8080/1/balance/0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045/0xdAC17F958D2ee523a2206206994597C13D831ec7
```

### SSE Balances Stream

Subscribe to real-time balance updates for all tokens.

```bash
curl -N http://localhost:8080/sse/{chain_id}/balances/{owner}
```

**Example (Ethereum mainnet):**
```bash
curl -N http://localhost:8080/sse/1/balances/0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Example (Sepolia testnet):**
```bash
curl -N http://localhost:8080/sse/11155111/balances/0xYourWalletAddress
```

**SSE Events:**

| Event | Description |
|-------|-------------|
| `balances` | Full balance snapshot (sent every 60s) |
| `update_balance` | Single token balance update (on Transfer event) |
| `error` | Error message |

**Response format:**

```
event: balances
data: {"balances":{"0xToken1Address":"1000000","0xToken2Address":"500000"}}

event: update_balance
data: {"address":"0xTokenAddress","balance":"1500000"}
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `HTTP_BIND` | Server bind address | `0.0.0.0:8080` |
| `ETH_RPC` | Ethereum HTTP RPC URL | - |
| `ETH_WC_RPC` | Ethereum WebSocket RPC URL | - |
| `ARBITRUM_RPC` | Arbitrum HTTP RPC URL | - |
| `SEPOLIA_RPC` | Sepolia HTTP RPC URL | - |
| `SEPOLIA_WC_RPC` | Sepolia WebSocket RPC URL | - |
| `TOKEN_LIST_PATH` | Path to token list config | `configs/tokens_list.json` |
| `MULTICALL_ADDRESS` | Multicall3 contract address | - |

## Quick Start

### Local Development

```bash
# Set environment variables
export ETH_RPC=https://eth.llamarpc.com
export ETH_WC_RPC=wss://eth.drpc.org

# Run
cargo run
```

### Docker

```bash
# Build
docker-compose build

# Run
docker-compose up -d

# View logs
docker-compose logs -f
```

## Chain IDs

| Network | Chain ID |
|---------|----------|
| Ethereum Mainnet | 1 |
| Arbitrum One | 42161 |
| Sepolia Testnet | 11155111 |

## Project Structure

```
src/
├── main.rs              # Entry point
├── args.rs              # CLI arguments
├── app_state.rs         # Application state
├── api/                 # HTTP handlers
│   ├── balance.rs       # Single balance endpoint
│   ├── balances.rs      # SSE balances stream
│   └── tokens_list.rs   # Token list endpoint
├── config/              # Configuration
├── evm/                 # EVM types (networks, tokens, ERC20)
├── routes/              # Router setup
├── services/            # Business logic
├── infra/               # Infrastructure (providers)
└── tracing/             # Logging setup
```

## License

MIT


