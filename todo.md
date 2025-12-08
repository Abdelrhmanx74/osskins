# Back End ---------------------

- [x] Fix build issues
- [x] State for user selects
- [-] Cache update with github / the json's update when there is a git commit
- [x] Refine LCU integration: Detect game state (champ select, in-game) for context-aware actions (e.g., auto-inject on game start, pre-game pop up).
- [x] Handle errors gracefully and provide user feedback on a prod level.
- [x] Custom skins tab. as in a whole different app tab with a dialog that opens uploading the skin file and adding its champion. and the skin name, etc...
- [-] Inject skins on reconnect

- [x] Optimize performance for large champion/skin datasets.
- [x] Ensure `mod-tools.exe` is compiled on build and not placed manually
- [x] app cosmetics and name, icon, etc...
- [x] Consider using a structured format (like JSON) for configuration instead of `league_path.txt` if more settings are planned.
- [x] It should not stop the injecting if the user closed the game (waiting to reconnect) it should close when it turn from in game to lobby, etc..
- [x] The terminals that opens!! it should not!
- [x] Handle game modes (arena, swift play)
- [x] Better injection error handling and cleaning cases not just at the end of a game
- [ ] Panic in changing champs swift and aram, etc..
- [ ] i feel like the first injection after opening the app is always slower than the next and might even break. can you investigate why and how can i help it
- [ ] issue when picking the skin after locking in the champ in aram or selective modes 
- [ ] checkout the cached mods
- [ ] when open stay at prev tray state

# Front End ---------------------

- [x] Better front-end code
- [x] Logical loading/stale state

- [x] Add Theming
- [x] All contexts should be zustand or react not both at the same time

# UX ---------------------

- [x] Favorites champs logic
- [x] Add search/filtering capabilities for the champion/skin list.
- [-] Animations baby!

# Friend Skin Sharing Feature Planning ---------------------

## Feature Overview

- Allow users to invite friends to sync each other's skins for champions they're playing
- Friends can see other friend selected skins on his friend's champion automatically
- Communication through League of Legends LCU messages (no private server needed)
- Pair system - connect once, works automatically forever

## UX Approach: Direct Friend Connection

### Core Flow:

1. **One-time Pairing** - Connect with friend once (like Bluetooth pairing)
2. **Champion Lock-in Detection** - App detects when you lock in a champion
3. **Real-time Skin Sharing** - When you lock in a champion, send your skin to connected friends
4. **Auto-Apply** - When you receive a friend's skin, automatically inject it so you see their selection
5. **Zero Maintenance** - Works automatically forever after initial setup

### Connection Process:

1. User A selects User B from their League friends list
2. User A clicks "Connect" for skin sharing
3. App sends special LCU message to User B
4. User B's Osskins app detects message and shows notification
5. User B accepts → reply with info so Both apps can store connection locally in the config file
6. From then on, automatically share champion skin when locking in during games, if A picks a champ and locks it in you send that champ skin info in the lcu messages and when friend B selects a champ and lock in it sends A that message and only inject when both/all the friends are locked in and now you can inject thoso skins for each friend

## Technical Implementation

### Key LCU API Endpoints

The LCU API endpoints actually used in the friend sharing implementation, with real examples:

| Category          | Endpoint                                   | Method | Description                                   |
| ----------------- | ------------------------------------------ | ------ | --------------------------------------------- |
| Friends List      | `/lol-chat/v1/friends`                     | GET    | Retrieves the list of friends with chat info. |
| Friends List Alt  | `/lol-chat/v1/friend-groups`               | GET    | Alternative endpoint for friends list.        |
| Friends List Alt2 | `/lol-summoner/v1/summoners/me/friends`    | GET    | Another alternative friends endpoint.         |
| Friends List Alt3 | `/lol-social/v1/friends`                   | GET    | Social service friends endpoint.              |
| Conversations     | `/lol-chat/v1/conversations`               | GET    | Gets all chat conversations.                  |
| Send Message      | `/lol-chat/v1/conversations/{id}/messages` | POST   | Sends a message in a conversation.            |
| Get Messages      | `/lol-chat/v1/conversations/{id}/messages` | GET    | Retrieves messages from a conversation.       |
| Champion Select   | `/lol-champ-select/v1/session`             | GET    | Gets current champion select session data.    |
| Current Summoner  | `/lol-summoner/v1/current-summoner`        | GET    | Gets current summoner information.            |

#### Example Requests & Responses:

**1. Get Friends List:**

Response Example:

```json
[
  {
    "availability": "chat",
    "displayName": "FriendName#1234",
    "gameName": "FriendName",
    "gameTag": "1234",
    "id": "12345678-abcd-1234-efgh-123456789012",
    "isOnline": true,
    "lastSeenOnlineTimestamp": "2025-07-13T10:30:00.000Z",
    "lol": {
      "gameStatus": "inGame",
      "mapId": "11",
      "queueId": "420"
    },
    "name": "FriendName",
    "pid": "friend-chat-id-12345",
    "puuid": "puuid-12345-abcd-efgh",
    "statusMessage": "Playing League of Legends",
    "summonerId": 123456789
  }
]
```

**2. Send Skin Share Message:**

```http
POST https://127.0.0.1:{port}/lol-chat/v1/conversations/{friend_id}/messages
Authorization: Basic {base64_encoded_riot_token}
Content-Type: application/json

{
  "body": "OSSKINS_SHARE:{\"message_type\":\"skin_share\",\"champion_id\":81,\"champion_name\":\"Ezreal\",\"skin_id\":81021,\"skin_name\":\"Battle Academia Ezreal\",\"chroma_id\":81032,\"from_summoner\":\"\",\"from_summoner_name\":\"\",\"timestamp\":1694876543210,\"message_id\":\"uuid-12345-abcd\"}",
  "type": "chat"
}
```

Response:

```json
{
  "id": "message-uuid-12345",
  "timestamp": "2025-07-13T10:30:00.000Z",
  "status": "sent"
}
```

**3. Get Champion Select Session:**

```http
GET https://127.0.0.1:{port}/lol-champ-select/v1/session
Authorization: Basic {base64_encoded_riot_token}
```

Response Example:

```json
{
  "actions": [
    [
      {
        "actorCellId": 0,
        "championId": 81,
        "completed": true,
        "id": 1,
        "isAllyAction": true,
        "isInProgress": false,
        "pickTurn": 1,
        "type": "pick"
      }
    ]
  ],
  "allowBattleBoost": false,
  "allowDuplicatePicks": false,
  "allowLockedEvents": false,
  "allowRerolling": false,
  "allowSkinSelection": true,
  "benchChampions": [],
  "benchEnabled": false,
  "boostableSkinCount": 10,
  "chatDetails": {
    "chatRoomName": "championSelect-12345",
    "chatRoomPassword": "password123"
  },
  "counter": 30,
  "entitledFeatureState": {
    "additionalRerolls": 0,
    "unlockedSkinIds": [81000, 81001, 81021]
  },
  "gameId": 5555555555,
  "hasSimultaneousBans": true,
  "hasSimultaneousPicks": false,
  "isSpectating": false,
  "localPlayerCellId": 0,
  "lockedEventIndex": -1,
  "myTeam": [
    {
      "assignedPosition": "middle",
      "cellId": 0,
      "championId": 81,
      "championPickIntent": 81,
      "entitledFeatureType": "",
      "selectedSkinId": 81021,
      "spell1Id": 4,
      "spell2Id": 7,
      "summonerId": 123456789,
      "team": 1,
      "wardSkinId": 6
    }
  ],
  "phase": "FINALIZATION",
  "pickOrderSwaps": [],
  "recoveryCounter": 30,
  "rerollsRemaining": 2,
  "skipChampionSelect": false,
  "theirTeam": [],
  "timer": {
    "adjustedTimeLeftInPhase": 25000,
    "internalNowInEpochMs": 1694876543210,
    "isInfinite": false,
    "phase": "FINALIZATION",
    "totalTimeInPhase": 30000
  },
  "trades": []
}
```

**4. Get Current Summoner:**

```http
GET https://127.0.0.1:{port}/lol-summoner/v1/current-summoner
Authorization: Basic {base64_encoded_riot_token}
```

Response Example:

```json
{
  "accountId": 123456789,
  "displayName": "MyGameName#1234",
  "gameName": "MyGameName",
  "internalName": "MyGameName",
  "nameChangeFlag": false,
  "percentCompleteForNextLevel": 50,
  "privacy": "PUBLIC",
  "profileIconId": 4200,
  "puuid": "my-puuid-12345-abcd-efgh",
  "rerollPoints": {
    "currentPoints": 150,
    "maxRolls": 2,
    "numberOfRolls": 1,
    "pointsCostToRoll": 250,
    "pointsToReroll": 100
  },
  "summonerId": 123456789,
  "summonerLevel": 125,
  "tagLine": "1234",
  "unnamed": false,
  "xpSinceLastLevel": 500,
  "xpUntilNextLevel": 500
}
```

**5. Get Chat Messages:**

```http
GET https://127.0.0.1:{port}/lol-chat/v1/conversations/{friend_id}/messages
Authorization: Basic {base64_encoded_riot_token}
```

Response Example:

```json
[
  {
    "body": "OSSKINS_SHARE:{\"message_type\":\"skin_share\",\"champion_id\":81,\"champion_name\":\"Ezreal\",\"skin_id\":81021,\"skin_name\":\"Battle Academia Ezreal\",\"chroma_id\":81032,\"from_summoner\":\"123456789\",\"from_summoner_name\":\"FriendName#1234\",\"timestamp\":1694876543210,\"message_id\":\"uuid-12345-abcd\"}",
    "fromId": "friend-chat-id-12345",
    "fromSummonerId": 987654321,
    "id": "message-uuid-67890",
    "isHistorical": false,
    "timestamp": "2025-07-13T10:30:00.000Z",
    "type": "chat"
  }
]
```

#### Skin Sharing Message Protocol:

**Connection Request Message:**

```json
{
  "message_type": "skin_share_request",
  "champion_id": 0,
  "champion_name": "",
  "skin_id": 0,
  "skin_name": "",
  "chroma_id": null,
  "from_summoner": "123456789",
  "from_summoner_name": "MyGameName#1234",
  "timestamp": 1694876543210,
  "message_id": "uuid-connection-request-12345"
}
```

**Connection Response Message:**

```json
{
  "message_type": "skin_share_response",
  "champion_id": 0,
  "champion_name": "",
  "skin_id": 0,
  "skin_name": "",
  "chroma_id": null,
  "from_summoner": "987654321",
  "from_summoner_name": "FriendName#1234",
  "timestamp": 1694876543210,
  "message_id": "uuid-connection-accepted-98765"
}
```

**Skin Share Message:**

```json
{
  "message_type": "skin_share",
  "champion_id": 81,
  "champion_name": "Ezreal",
  "skin_id": 81021,
  "skin_name": "Battle Academia Ezreal",
  "chroma_id": 81032,
  "from_summoner": "123456789",
  "from_summoner_name": "MyGameName#1234",
  "timestamp": 1694876543210,
  "message_id": "uuid-skin-share-67890"
}
```

All messages are sent as chat messages with the prefix `OSS:` followed by the JSON payload.

### Implementing Real-Time Champion Skin Sharing

The actual implementation uses these LCU endpoints with the following workflow:

1. **Fetching the friend list** - Tries multiple endpoints in order:

   - Primary: `GET /lol-chat/v1/friends`
   - Fallback: `/lol-chat/v1/friend-groups`, `/lol-summoner/v1/summoners/me/friends`, `/lol-social/v1/friends`
   - Parses friend data to extract `id` (for chat), `displayName`, `availability`, etc.

2. **Monitoring champion lock-in events** - Polls `GET /lol-champ-select/v1/session` every few seconds:

   - Checks `myTeam[localPlayerCellId].championId > 0` to detect lock-in
   - Emits `champion-locked-in` event when state changes
   - Clears previous skin shares when switching champions

3. **Sending champion skin data** via `POST /lol-chat/v1/conversations/{friend_id}/messages`:

   - Message format: `"OSSKINS_SHARE:" + JSON.stringify(skinShareMessage)`
   - Contains: champion_id, skin_id, chroma_id, champion_name, skin_name, timestamp, message_id
   - Automatically sent to all connected friends when champion is locked in

4. **Receiving friend's skins** with `GET /lol-chat/v1/conversations/{friend_id}/messages`:

   - Polls messages every few seconds for `OSSKINS_SHARE:` prefix
   - Parses JSON payload and stores friend skin data

### Champion Lock-in Sharing

**How it works**:

1. User A locks in any champion → App sends A's skin to User/s B
2. User B locks in any champion → App sends B's skin to User A/s
3. Both users automatically inject the received skin when both/all are locked in
4. Result: You see your friend's skin selection, they see yours

### Connection Request Dialog:

```
┌─────────────────────────────────────┐
│ 🔗 Connect with Friend              │
├─────────────────────────────────────┤
│ Select a friend to connect with:    │
│                                     │
│ 🟢 Jake_Gaming (Online)             │
│ 🟡 Sarah_ADC (Away)                 │
│ 🟢 MikeSupport (Online)             │
│                                     │
│         │
└─────────────────────────────────────┘
```

this should be trigger by a menu item in the main app menu next to the update data button and on click it opens a dialog

### Status Indicator:

```
┌─────────────────────────────┐
│ 🎮 Osskins    👥 2 Connected│
└─────────────────────────────┘
```

### Real-Time Skin Sharing Notifications:

**Purpose**: Show users when friends lock in champions and their skins are being shared/applied.

**Notification Examples**:

- [x] "🎨 Jake locked in Ahri with KDA skin - Applied!"
- [x] "✨ Your Spirit Blossom Yasuo sent to Mike"
- [x] "🔥 Received Sarah's Project Jhin - Injected!"

**Requirements**:

- Show notification when friend locks in any champion
- Show notification when your skin is sent to friends
- Show notification when friend's skin is received
- Notifications should be brief and non-intrusive
- Include friend name, champion, and skin details
