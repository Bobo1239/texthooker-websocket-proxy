# `texthooker_websocket_proxy`

Part of my Japanese sentence mining setup. Probably not useful for anybody besides me...

This is needed for two reasons:
- Unified port for all sources; especially important since some don't allow port configuration (e.g. textractor-websocket 6677; agent 9001)
- Firefox doesn't allow constant retries to a WebSocket server so we just keep this running so there's always something to connect to (https://stackoverflow.com/a/59548772)
