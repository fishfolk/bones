# Matchmaking

For online matchmaking, we use a centralized matchmaking server to connect peers to each-other and to forward the peers' network traffic. All connections utilize UDP and the QUIC protocol.

The matchmaking server is implemented in the [`bones_matchmaker`] crate. It binds to a single UDP port and listens for client connections. QUIC's multiplexing capabilities allow the server to handle any number of clients on this single port.

Once a match starts, client traffic is proxied through the matchmaking server. While not true peer-to-peer networking, clients logically send messages to each other, with the server acting as an intermediary.

Pros of this approach:
- Reduced connections per peer (one connection to the matchmaker only)
- Client IP addresses are hidden from each other
- Easier to bypass firewalls and NATs
- Simplified connection process

Cons:
- Increased server bandwidth usage
- Additional network hop between peers, potentially increasing latency

This design doesn't preclude future support for true peer-to-peer connections or LAN games without a matchmaker.

[`bones_matchmaker`]: https://github.com/fishfolk/bones/tree/main/crates/bones_matchmaker

## Matchmaking Protocol

### Initial Connection

1. The client connects to the matchmaking server.
2. The client sends a [`RequestMatchmaking`][crate::external::bones_matchmaker_proto::MatchmakerRequest::RequestMatchmaking] message over a reliable channel.
3. This message contains [`MatchInfo`] with:
   - The desired number of players
   - A `game_id` to identify the game
   - `match_data` (arbitrary bytes for game mode, parameters, etc.)
   - `player_idx_assignment` to specify how player ids should be assigned (ie. randomly assign a side for a pvp match)

Players must specify identical `MatchInfo` to be matched together. The `match_data` ensures players are connected for the same game mode and version.

### Waiting for Players

1. The server responds with an [`Accepted`][crate::external::bones_matchmaker_proto::MatchmakerResponse::Accepted] message.
2. While waiting, the server may send [`MatchmakingUpdate`][crate::external::bones_matchmaker_proto::MatchmakerResponse::MatchmakingUpdate] messages with the current player count.
3. When the desired number of players is reached, the server sends a [`Success`][crate::external::bones_matchmaker_proto::MatchmakerResponse::Success] message containing:
   - A `random_seed` for deterministic random number generation
   - The client's `player_idx`
   - The total `player_count`
   - A list of `player_ids` with their network addresses

### In the Match

Once all players have received the `Success` message, the matchmaker enters proxy mode:

1. Clients send [`SendProxyMessage`][crate::external::bones_matchmaker_proto::SendProxyMessage]s to the server, specifying a target client (or all clients) and the message data.
2. The server forwards these as [`RecvProxyMessage`][crate::external::bones_matchmaker_proto::RecvProxyMessage]s to the target client(s), including the sender's player index.

The matchmaker supports both reliable and unreliable message forwarding, allowing the game to implement its preferred synchronization protocol.