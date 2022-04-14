# Validators Instructions for Setting Up Steward

Steward is an application intended for developers and validators on the Sommelier network.

It can run as a server for forearding cellar contract calls from Strategy Providers through the Sommelier chain, or it can run in test mode to directly interact with Ethereum contracts as a single signer.

It integrates the full functionality of gorc for operating as an orchestrator and relayer of [Gravity bridge](https://github.com/PeggyJV/gravity-bridge/) messages between the Ethereum and Cosmos chains.

Stewards works in conjunction with the Orchestrator, so both processes must be running to fully participate in Cellar management.

## Quickstart

To start Steward, simply run

```bash
steward -c [path to your config toml] start
```

## Setting Up Steward

In this section, let’s explore setting up steward for validators. There are two ways that validators will use Steward:

1. Running Steward as a gRPC server for relaying SP contract calls through the chain. If a validator is not running this server, it cannot participate in Cellar management.
2. Manually scheduling contract calls when the validator set needs to coordinate a function  call on a cellar. An example of this would be transferring a Cellar’s accrued fees to a module account on chain.

### Running Steward as a server

Steward runs on every Validator in the Sommelier Validator set. It runs a server to which Strategy Providers (SPs) send requests whenever they determine that the market has changed enough to warrant action. The request payload contains everything needed to make a *cork*: a signed combination of a cellar ID and an ABI encoded contract call. When Steward receives a submission from the SP, it validates the target cellar ID, build a cork, and submits it to the Cork module on chain.

Here is an ***example*** TOML file with the minimum required configuration fields to run Steward as a server and facilitate Cellar operations. *Please fill in with your own values*:

```toml
[cork]
# Before Steward forwards a function call to the chain, it checks
# that the target contract address is in fact a cellar approved
# by governance. To speed up this check, steward frequently queries
# and caches a list of all approved cellar addresses. This value
# determines how frequently (in seconds) steward makes this query.
cache_refresh_period = 60           # default: 60

[cosmos]
# Your sommelier gRPC endpoint
grpc = "http://localhost:9090"      # default: "http://localhost:9090"

# The bech32 prefix for address strings
prefix = "somm"                     # default: "somm"

[cosmos.gas_price]
amount = 0.0                        # default: 0.0
denom = "usomm"                     # default "usomm"

[keys]
# The name of key in the keystore to be used for signing transactions.
# This should be the same key for Orchestrator and Steward.
delegator_key = "mykey"

# The on-disk keystore where Steward-managed keys are stored
keystore = "/some/path"             # default "/tmp/keystore"

[server]
# The address of the Steward gRPC server
address = "0.0.0.0"                 # default "0.0.0.0"

# The port of the Steward gRPC server
port = 5734                         # default 5734

# The root of trust that signed the Strategy Provider's client certificate.
client_ca_cert_path = "./truststore/sp_client_ca.crt"

# The server's cert to offer the SP client to establish two-way trust
server_cert_path = "./server.crt"

# The key used to generate the server cert
server_key_path = "./server_key_pkcs8.pem"
```