# Mock LCU Server

This is a mock implementation of the League Client Update (LCU) server, designed for testing and developing dashboards or integrations that interact with the League client and party mode features.

## Features
- Mocks LCU endpoints for friends, conversations, and gameflow phase
- Mocks party mode endpoints for pairing requests, responses, and skin sharing
- Real-time updates via Socket.io
- In-memory state for phase, friends, conversations, and party mode

## Endpoints
- `GET /lol-chat/v1/friends` — Returns fake friend list
- `GET /lol-chat/v1/conversations` — Returns fake conversations
- `GET /lol-gameflow/v1/gameflow-phase` — Returns current phase
- `POST /mock-lcu/set-phase` — Set the current LCU phase
- `GET /party-mode/friends` — Returns fake friend list
- `POST /party-mode/pairing-request` — Send a pairing request
- `POST /party-mode/pairing-response` — Respond to a pairing request
- `POST /party-mode/skin-share` — Share a skin with a friend

## Real-time Events
- Socket.io emits events for phase changes, party mode pairing, and skin shares

## Usage

1. Install dependencies:
   ```sh
   npm install
   ```
2. Start the server:
   ```sh
   npm start
   ```
3. The server will run on [http://localhost:5175](http://localhost:5175)

You can now point your dashboard or app to this mock server for development and testing. 