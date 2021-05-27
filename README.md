# Tetsy Orchestrator

**NB**: This is a prototype, an updated version will be published soon, with a few
more features, improved documentation and code.

Tetsy Orchestrator enables greedy peering (and, potentially, other strategies)
between Tetsy nodes. It operates as an external program and communicates
with its local parity node over JSON-RPC.

Orchestrator announces its local node regularly and listens for other announcements
to add nodes its own Parity node doesn't peer with. Every node that wants to participate
in greedy peering must run Orchestrator alongside.

Currently, to be used, a [workaround version of Parity with Whisper issues fixed](https://github.com/poanetwork/parity/tree/disable-parity-whisper-extensions)
needs to be used. That is, until it's fixed.

A node to be managed by Orchestrator needs to enable `tetsy_set` JSON-RPC API and Whisper. The node's port should be available through
the node's public IP. Orchestrator WILL try to "call back home" and will fail to start if this attempt will be unsuccessful.

One has to provide a configuration for the node, such as:

```toml
[node_announcement_topic]
type = "string"
topic = "announcement"
```

This defines which Whisper topic will be used to announce node's enode across the network. A binary
topic name can be used as well:

```toml
[node_announcement_topic]
type = "binary"
topic = "0x...."
```

By default, Orchestrator will announce the node every 30 seconds, and this can be configured as well:

```toml
node_announcement_frequency = 60
```

By default, Orchestrator will try to figure out node's public IP using http://checkip.amazonaws.com, however,
this can be changed to a manual IP address:

```toml
[address]
type = "manual"
ip = "..."
```

It's also possible to specify different JSON-RPC endpoint (other than http://localhost:8545):

```toml
tetsy_node = "http://host:8545"
```

By default, Orchestrator will try to find tetsy-orchestrator.toml, but this can be changed with a `-c/--config` argument.

## Notes

In some cases, it is useful to see if Parity Orchestrator is doing anything. It's typically logging
"Polling announcements" into a trace log, which is normally disabled. For that reason, there's `reveal_trace_every_secs`
configuration parameter. It will show this log statement every 10 seconds by default. Setting it to 0 will disable it.

Orchestrator also keeps a log of enodes it added (format: "iso8601 enode" lines), configured with
`orchestrator_additions_file` configuration parameter ("tetsy-orchestrator-nodes" by default)
