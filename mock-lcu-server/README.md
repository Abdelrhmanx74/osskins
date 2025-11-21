# Mock LCU Server Notes

This mock server now emulates the endpoints used by Party Mode more realistically:

- GET /lol-chat/v1/friends — returns friends with pid/puuid and online availability
- GET /lol-chat/v1/conversations — returns conversations with pid and id
- GET /lol-chat/v1/conversations/:id/messages — messages contain numeric `id` like the real LCU
- POST /lol-chat/v1/conversations — creates chat by pid
- POST /lol-chat/v1/conversations/:id/messages — accepts `body` and `type`
- GET /lol-summoner/v1/current-summoner — returns the local user
- GET /lol-gameflow/v1/session and /lol-gameflow/v1/gameflow-phase — basic phase simulation
- GET /lol-champ-select/v1/session — reports local pick completion
- GET /lol-lobby/v2/lobby — reports current party members (user + selected friends)

Test helpers:

- POST /test/toggle-friend-sharing — toggle whether a friend “shares”
- POST /test/friend-lock-skin — locks friend skin and emits OSS:skin_share
- POST /test/local-player-lock-skin — simulates local share
- POST /test/toggle-friend-in-lobby — toggles whether a friend is in your party
- POST /test/set-selected-champion — sets champ select preview

Tips:

- Party Mode waits only for paired friends who are in your current party (lobby or champ select).
- If you’re solo or no paired friends are in the party, injection proceeds immediately.
- Numeric message IDs are used to better match real LCU behavior.

# Mock LCU Server for Party Mode Testing

This mock server simulates the League of Legends Client Update (LCU) API to test the party mode feature of the OSS application.

## Features

### Implemented LCU Endpoints

- **GET /lol-chat/v1/friends** - Returns mock friends list
- **GET /lol-chat/v1/conversations** - Returns chat conversations
- **GET /lol-chat/v1/conversations/:id/messages** - Returns messages for a conversation
- **POST /lol-chat/v1/conversations/:id/messages** - Send a message to a conversation
- **POST /lol-chat/v1/conversations** - Create a new conversation
- **GET /lol-summoner/v1/current-summoner** - Returns current user info
- **GET /lol-gameflow/v1/session** - Returns game flow session data
- **GET /lol-gameflow/v1/gameflow-phase** - Returns current game phase
- **GET /lol-champ-select/v1/session** - Returns champion select session data

### Party Mode Simulation

The server correctly simulates party mode by:

1. Processing messages with "OSS:" prefix in chat conversations
2. Parsing JSON party mode messages (pairing_request, pairing_response, skin_share)
3. Emitting appropriate Socket.IO events to simulate frontend notifications
4. Providing a web dashboard for easy testing

### Test Endpoints

- **POST /test/send-pairing-request** - Simulate receiving a pairing request
- **POST /test/respond-to-request** - Simulate a friend accepting/declining your request
- **POST /test/share-skin** - Simulate receiving a skin share

## Usage

1. **Start the server:**

   ```bash
   cd mock-lcu-server
   npm install
   node index.js
   ```

2. **Access the dashboard:**
   Open https://localhost:56174/dashboard in your browser

3. **Configure your app:**
   Point your OSS application to use `https://localhost:56174` as the LCU endpoint

4. **Test party mode:**
   - Use the dashboard to simulate pairing requests from friends
   - Test accepting/declining requests
   - Test skin sharing functionality
   - Monitor the event log to see what's happening

## How Party Mode Works

The party mode feature uses the LCU chat system to exchange messages between friends:

1. **Pairing Requests**: Sent as chat messages with format `OSS:{"message_type":"pairing_request","data":{...}}`
2. **Responses**: Sent as chat messages with format `OSS:{"message_type":"pairing_response","data":{...}}`
3. **Skin Sharing**: Sent as chat messages with format `OSS:{"message_type":"skin_share","data":{...}}`

The mock server correctly simulates this by:

- Creating conversations between the user and friends
- Processing OSS-prefixed messages
- Emitting events that the frontend listens for
- Maintaining conversation history

## Mock Data

The server includes two mock friends:

- **friend1#NA1** (ID: 3458743863674223)
- **sisi#NA1** (ID: 3458743863674208)

And a mock current user:

- **rogolax** (ID: 148331403)

You can test all party mode functionality with these mock friends.

## Development Notes

This mock server replaces the original custom party-mode endpoints with proper LCU endpoints that match what the real League client provides. The key insight was that party mode works through the chat system, not through custom endpoints.

The server provides both Socket.IO events for real-time updates and proper REST endpoints that match the LCU API structure.
