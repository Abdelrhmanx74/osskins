const express = require("express");
const cors = require("cors");
const bodyParser = require("body-parser");
const https = require("https");
const fs = require("fs");
const { Server } = require("socket.io");
const say = require("say");

const app = express();
const server = https.createServer(
  {
    key: fs.readFileSync("server.key"),
    cert: fs.readFileSync("server.cert"),
  },
  app
);
const io = new Server(server, { cors: { origin: "*" } });

app.use(cors());
app.use(bodyParser.json());

// Health check
app.get("/health", (req, res) => {
  res.json({ status: "ok" });
});

// --- Mock Data ---
const user = {
  summonerId: 148331403,
  displayName: "You",
  gameName: "You",
  gameTag: "Local",
  pid: "5db00f0b-8d04-5fe9-8fdb-5f768cbced5b@eu1.pvp.net",
  puuid: "5db00f0b-8d04-5fe9-8fdb-5f768cbced5b",
};

const friends = [
  {
    summonerId: 3458743863674223,
    gameName: "friend1",
    gameTag: "NA1",
    displayName: "friend1#NA1",
    availability: "online",
    puuid: "0e624b10-3d3e-5222-ac66-c0dd162d83e8",
    pid: "0e624b10-3d3e-5222-ac66-c0dd162d83e8@eu1.pvp.net",
    isSharing: false,
    selectedSkin: null,
    isLockedIn: false,
  },
  {
    summonerId: 3458743863674208,
    gameName: "sisi",
    gameTag: "NA1",
    displayName: "sisi#NA1",
    availability: "online",
    puuid: "d56d6f6e-5540-5a46-9724-116f99cf98f0",
    pid: "d56d6f6e-5540-5a46-9724-116f99cf98f0@eu1.pvp.net",
    isSharing: false,
    selectedSkin: null,
    isLockedIn: false,
  },
];

// Available skins for simulation
const availableSkins = [
  {
    championId: 81,
    skinId: 81021,
    skinName: "Ezreal Pulsefire",
    champName: "ezreal",
  },
  { championId: 24, skinId: 24022, skinName: "Jax Empyrean", champName: "jax" },
  { championId: 103, skinId: 103015, skinName: "Ahri K/DA", champName: "ahri" },
  {
    championId: 157,
    skinId: 157001,
    skinName: "Yasuo High Noon",
    champName: "yasuo",
  },
  {
    championId: 39,
    skinId: 39003,
    skinName: "Irelia Frostblade",
    champName: "irelia",
  },
];

// LCU Chat data
let conversations = {};
let messageIdCounter = 1;
let lcuState = "Lobby"; // Lobby, ChampSelect, Matchmaking, InProgress, Reconnect
let champSelectSession = {};
let selectedChampionId = null; // Dynamic champion selection
let lobbyMembers = new Set(); // Track current lobby member summonerIds (numbers)
// Swift Play: allow selecting up to two champions in lobby
let swiftPlaySelections = []; // array of championIds
// ARAM: when enabled, auto-assign random champs to everyone and auto-share
let aramAutoShare = false;

// Socket.IO connection
io.on("connection", (socket) => {
  console.log("Client connected:", socket.id);
  socket.on("disconnect", () => {
    console.log("Client disconnected:", socket.id);
  });
});

// Helper function to get or create conversation ID
const getConversationId = (friendPid) => {
  if (!conversations[friendPid]) {
    conversations[friendPid] = {
      id: `conversation_${Object.keys(conversations).length + 1}`,
      pid: friendPid,
      messages: [],
      type: "chat",
    };
  }
  return conversations[friendPid].id;
};

// Helper function to process party mode messages (simplified)
const processPartyModeMessage = (body, fromSummonerId) => {
  if (!body.startsWith("OSS:")) return;

  try {
    const messageData = JSON.parse(body.substring(4)); // Remove "OSS:" prefix
    console.log("[Party Mode] Processing message:", messageData);

    switch (messageData.message_type) {
      case "skin_share":
        const skinShare = messageData.data;
        console.log("[Party Mode] Received skin share:", skinShare);
        io.emit("party-mode-skin-received", skinShare);
        break;
    }
  } catch (error) {
    console.error("[Party Mode] Failed to parse message:", error);
  }
};

// --- LCU Endpoints ---
// Get friends list (actual LCU endpoint)
app.get("/lol-chat/v1/friends", (req, res) => {
  console.log("[LCU] GET /lol-chat/v1/friends");
  // Return minimal realistic friend objects including pid and puuid
  res.json(
    friends.map((f) => ({
      summonerId: f.summonerId,
      gameName: f.gameName,
      gameTag: f.gameTag,
      displayName: f.displayName,
      availability: f.availability,
      puuid: f.puuid,
      pid: f.pid,
    }))
  );
  console.log("[LCU] Response:", friends);
});

// Get conversations (actual LCU endpoint)
app.get("/lol-chat/v1/conversations", (req, res) => {
  console.log("[LCU] GET /lol-chat/v1/conversations");
  const conversationList = Object.values(conversations);
  res.json(conversationList);
  console.log("[LCU] Response:", conversationList);
});

// Get messages for a conversation (actual LCU endpoint)
app.get("/lol-chat/v1/conversations/:conversationId/messages", (req, res) => {
  const { conversationId } = req.params;
  console.log(
    `[LCU] GET /lol-chat/v1/conversations/${conversationId}/messages`
  );

  const conversation = Object.values(conversations).find(
    (c) => c.id === conversationId
  );
  if (!conversation) {
    return res.status(404).json({ error: "Conversation not found" });
  }

  res.json(conversation.messages);
  console.log(`[LCU] Response: ${conversation.messages.length} messages`);
});

// Send message to conversation (actual LCU endpoint)
app.post("/lol-chat/v1/conversations/:conversationId/messages", (req, res) => {
  const { conversationId } = req.params;
  const { body, type = "chat" } = req.body;

  console.log(
    `[LCU] POST /lol-chat/v1/conversations/${conversationId}/messages`
  );
  console.log(`[LCU] Message body: ${body}`);

  const conversation = Object.values(conversations).find(
    (c) => c.id === conversationId
  );
  if (!conversation) {
    return res.status(404).json({ error: "Conversation not found" });
  }

  const message = {
    // Real LCU can use numeric IDs; keep as number for realism
    id: messageIdCounter++,
    body,
    type,
    fromSummonerId: user.summonerId.toString(),
    timestamp: Date.now(),
  };

  conversation.messages.push(message);

  // Emit socket event for dashboard chat log (outgoing message from local user)
  try {
    // Try to resolve friend name from conversation pid
    const friend = friends.find((f) => f.pid === conversation.pid);
    io.emit("chat-message", {
      direction: "sent",
      body,
      to: friend ? friend.displayName : conversation.pid,
      to_id: friend ? friend.summonerId.toString() : undefined,
      conversationId: conversation.id,
      timestamp: message.timestamp,
    });
  } catch { }

  // Process party mode messages
  processPartyModeMessage(body, user.summonerId.toString());

  res.json(message);
  console.log(`[LCU] Message sent successfully`);
});

// Create new conversation (actual LCU endpoint)
app.post("/lol-chat/v1/conversations", (req, res) => {
  console.log("[LCU] POST /lol-chat/v1/conversations");
  console.log("[LCU] Request body:", req.body);

  const { pid, type = "chat" } = req.body;

  if (!pid) {
    return res.status(400).json({ error: "pid is required" });
  }

  const conversationId = getConversationId(pid);
  const conversation = conversations[pid];

  res.json(conversation);
  console.log("[LCU] Created conversation:", conversation);
});

// Mock current summoner endpoint
app.get("/lol-summoner/v1/current-summoner", (req, res) => {
  console.log("[LCU] GET /lol-summoner/v1/current-summoner");
  const currentSummoner = {
    accountId: user.summonerId,
    summonerId: user.summonerId,
    puuid: user.puuid,
    displayName: user.displayName,
    gameName: user.gameName,
    tagLine: user.gameTag,
    summonerLevel: 100,
    profileIconId: 1,
  };
  res.json(currentSummoner);
  console.log("[LCU] Response:", currentSummoner);
});

// --- Test Endpoints for Dashboard ---
// Toggle friend sharing status
app.post("/test/toggle-friend-sharing", (req, res) => {
  const { friendIndex = 0 } = req.body;
  const friend = friends[friendIndex];

  if (!friend) {
    return res.status(400).json({ error: "Friend not found" });
  }

  friend.isSharing = !friend.isSharing;
  console.log(
    `[TEST] ${friend.displayName} sharing toggled to: ${friend.isSharing}`
  );

  // Reset lock-in status when sharing is turned off
  if (!friend.isSharing) {
    friend.isLockedIn = false;
    friend.selectedSkin = null;
  }

  io.emit("friend-sharing-updated", {
    friendIndex,
    friend: friend,
  });

  res.json({
    success: true,
    message: `${friend.displayName} sharing ${friend.isSharing ? "enabled" : "disabled"
      }`,
    friend: friend,
  });
});

// Set all friends sharing ON/OFF
app.post("/test/friends/sharing", (req, res) => {
  const { enabled = true } = req.body || {};
  friends.forEach((f) => {
    f.isSharing = !!enabled;
    if (!enabled) {
      f.isLockedIn = false;
      f.selectedSkin = null;
    }
  });
  io.emit("friend-sharing-updated", { bulk: true, enabled });
  res.json({ success: true, enabled });
});

// Put all friends in/out of lobby
app.post("/test/friends/lobby", (req, res) => {
  const { inLobby = true } = req.body || {};
  if (inLobby) {
    friends.forEach((f) => lobbyMembers.add(f.summonerId));
  } else {
    lobbyMembers.clear();
  }
  res.json({ success: true, inLobby });
});

// Lock in a skin for a friend
app.post("/test/friend-lock-skin", (req, res) => {
  const { friendIndex = 0, skinIndex = 0 } = req.body;
  const friend = friends[friendIndex];
  const skin = availableSkins[skinIndex];

  if (!friend) {
    return res.status(400).json({ error: "Friend not found" });
  }

  if (!skin) {
    return res.status(400).json({ error: "Skin not found" });
  }

  if (!friend.isSharing) {
    return res.status(400).json({ error: "Friend is not sharing" });
  }

  friend.selectedSkin = skin;
  friend.isLockedIn = true;
  // Ensure friend is considered in your party for the session
  lobbyMembers.add(friend.summonerId);

  console.log(
    `[TEST] ${friend.displayName} locked in ${skin.skinName} - sending skin share message`
  );

  // Create conversation if it doesn't exist
  const conversationId = getConversationId(friend.pid);

  // Create the skin share message (exactly like a real friend would send)
  // Use fantome_path that maps to files the app likely has to avoid skips during tests
  const normalized = skin.skinName.toLowerCase().replace(/[^a-z0-9]/g, "_");
  const friendFantomeCandidates = [
    `/${skin.champName}/${normalized}.zip`,
    // Prefer built-in demo zips we ship in champions folder when available
    `${skin.champName}/${normalized}.zip`,
  ];
  const skinShare = {
    from_summoner_id: friend.summonerId.toString(),
    from_summoner_name: friend.displayName,
    champion_id: skin.championId,
    skin_id: skin.skinId,
    skin_name: skin.skinName,
    chroma_id: null, // Mock friends don't use chromas for simplicity
    fantome_path: friendFantomeCandidates[0],
    timestamp: Date.now(),
  };

  const partyMessage = {
    message_type: "skin_share",
    data: skinShare,
  };

  const messageBody = `OSS:${JSON.stringify(partyMessage)}`;

  // Add message to conversation (simulate receiving it from friend)
  const message = {
    id: messageIdCounter++,
    body: messageBody,
    type: "chat",
    fromSummonerId: friend.summonerId.toString(),
    timestamp: Date.now(),
  };

  const conversation = Object.values(conversations).find(
    (c) => c.id === conversationId
  );
  conversation.messages.push(message);

  // Process the message (this triggers the same logic as a real skin share)
  processPartyModeMessage(messageBody, friend.summonerId.toString());

  // Emit socket event for dashboard chat log (incoming message from friend)
  io.emit("chat-message", {
    direction: "received",
    body: messageBody,
    from: friend.displayName,
    from_id: friend.summonerId.toString(),
    conversationId,
    timestamp: message.timestamp,
  });

  io.emit("friend-locked-in", {
    friendIndex,
    friend: friend,
    skin: skin,
  });

  res.json({
    success: true,
    message: `${friend.displayName} locked in ${skin.skinName} and sent skin share`,
    friend: friend,
  });
});

// Send your own skin lock-in to friends (simulate local player sharing)
app.post("/test/local-player-lock-skin", (req, res) => {
  const { skinIndex = 0 } = req.body;
  const skin = availableSkins[skinIndex];

  if (!skin) {
    return res.status(400).json({ error: "Skin not found" });
  }

  console.log(
    `[TEST] Local player (You) locked in ${skin.skinName} - sending to all friends`
  );

  // Send skin share message to each friend (simulate sending OSS messages to friends)
  friends.forEach((friend, index) => {
    if (!friend.isSharing) return; // Only send to friends who are sharing

    const conversationId = getConversationId(friend.pid);

    // Create the skin share message from local player
    const skinShare = {
      from_summoner_id: user.summonerId.toString(),
      from_summoner_name: user.displayName,
      champion_id: skin.championId,
      skin_id: skin.skinId,
      skin_name: skin.skinName,
      chroma_id: null, // Mock user doesn't use chromas for simplicity
      fantome_path: `mock_local_skins/${skin.skinName
        .toLowerCase()
        .replace(/[^a-z0-9]/g, "_")}.fantome`, // Mock fantome path
      timestamp: Date.now(),
    };

    const partyMessage = {
      message_type: "skin_share",
      data: skinShare,
    };

    const messageBody = `OSS:${JSON.stringify(partyMessage)}`;

    // Add message to conversation (simulate sending it to friend)
    const message = {
      id: messageIdCounter++,
      body: messageBody,
      type: "chat",
      fromSummonerId: user.summonerId.toString(),
      timestamp: Date.now(),
    };

    const conversation = Object.values(conversations).find(
      (c) => c.id === conversationId
    );
    conversation.messages.push(message);

    console.log(`[TEST] Sent ${skin.skinName} to ${friend.displayName}`);

    // Emit socket event for dashboard chat log (outgoing message from local user)
    io.emit("chat-message", {
      direction: "sent",
      body: messageBody,
      to: friend.displayName,
      to_id: friend.summonerId.toString(),
      conversationId,
      timestamp: message.timestamp,
    });
  });

  // Do NOT process your own message locally - you're the sender, not a receiver!

  io.emit("local-player-locked-in", {
    skin: skin,
    sentToFriends: friends.filter((f) => f.isSharing).length,
  });

  res.json({
    success: true,
    message: `You locked in ${skin.skinName} and sent to ${friends.filter((f) => f.isSharing).length
      } friends`,
    skin: skin,
  });
});

// Get current party mode state
app.get("/test/party-state", (req, res) => {
  res.json({
    friends: friends,
    availableSkins: availableSkins,
    currentPhase: lcuState,
    selectedChampionId: selectedChampionId,
    swiftPlaySelections: swiftPlaySelections,
    aramEnabled: aramAutoShare,
  });
});

// Clear party state: set to Lobby and remove all messages
app.post("/test/clear-party-state", (req, res) => {
  console.log("[TEST] Clearing party state and messages -> Lobby");

  // Set LCU state to Lobby
  lcuState = "Lobby";

  // Clear selected champion
  selectedChampionId = null;

  // Clear conversation messages
  Object.keys(conversations).forEach((k) => {
    conversations[k].messages = [];
  });

  // Reset friend lock states and lobby membership
  friends.forEach((f) => {
    f.isLockedIn = false;
    f.selectedSkin = null;
  });
  lobbyMembers.clear();

  // Emit updates to connected dashboards/clients
  io.emit("lcu_state", { state: lcuState });
  io.emit("party-state-cleared", { success: true });

  res.json({ success: true, message: "Party state cleared and set to Lobby" });
});

// Set your selected champion (what champ select shows)
app.post("/test/set-selected-champion", (req, res) => {
  const { championId } = req.body;

  if (championId === undefined || championId === null) {
    selectedChampionId = null;
    console.log(`[TEST] Cleared champion selection`);
  } else {
    const champion = availableSkins.find((s) => s.championId === championId);
    selectedChampionId = championId;
    console.log(
      `[TEST] Set selected champion to ${championId} (${champion ? champion.skinName.split(" ")[0] : "Unknown"
      })`
    );
  }

  res.json({
    success: true,
    selectedChampionId: selectedChampionId,
    message: selectedChampionId
      ? `Selected champion ${selectedChampionId}`
      : `Cleared champion selection`,
  });
});

// Lock in your champion and automatically share your skin (simulates the full flow)
app.post("/test/lock-in-champion", (req, res) => {
  const { championId } = req.body;

  if (!championId) {
    return res.status(400).json({ error: "championId is required" });
  }

  const skin = availableSkins.find((s) => s.championId === championId);
  if (!skin) {
    return res.status(400).json({ error: "No skin found for this champion" });
  }

  // Set the selected champion
  selectedChampionId = championId;
  console.log(
    `[TEST] You locked in ${skin.skinName} - automatically sending skin share to friends`
  );

  // Automatically send skin share to all sharing friends (like the real app would do)
  friends.forEach((friend, index) => {
    if (!friend.isSharing) return; // Only send to friends who are sharing

    const conversationId = getConversationId(friend.pid);

    // Create the skin share message from local player
    const skinShare = {
      from_summoner_id: user.summonerId.toString(),
      from_summoner_name: user.displayName,
      champion_id: skin.championId,
      skin_id: skin.skinId,
      skin_name: skin.skinName,
      chroma_id: null, // Mock user doesn't use chromas for simplicity
      fantome_path: `mock_local_skins/${skin.skinName
        .toLowerCase()
        .replace(/[^a-z0-9]/g, "_")}.fantome`, // Mock fantome path
      timestamp: Date.now(),
    };

    const partyMessage = {
      message_type: "skin_share",
      data: skinShare,
    };

    const messageBody = `OSS:${JSON.stringify(partyMessage)}`;

    // Add message to conversation (simulate sending it to friend)
    const message = {
      id: (messageIdCounter++).toString(),
      body: messageBody,
      type: "chat",
      fromSummonerId: user.summonerId.toString(),
      timestamp: Date.now(),
    };

    const conversation = Object.values(conversations).find(
      (c) => c.id === conversationId
    );
    conversation.messages.push(message);

    console.log(`[TEST] Sent ${skin.skinName} to ${friend.displayName}`);
  });

  io.emit("local-player-locked-in", {
    championId: championId,
    skin: skin,
    sentToFriends: friends.filter((f) => f.isSharing).length,
  });

  res.json({
    success: true,
    message: `You locked in ${skin.skinName} and sent to ${friends.filter((f) => f.isSharing).length
      } friends`,
    championId: championId,
    skin: skin,
  });
});

// LCU state transitions
app.get("/lcu/v1/state", (req, res) => {
  console.log("[LCU] GET /lcu/v1/state");
  res.json({ state: lcuState });
  console.log("[LCU] Response:", lcuState);
});

app.post("/lcu/v1/state", (req, res) => {
  const { state } = req.body;
  console.log("[LCU] POST /lcu/v1/state", req.body);
  lcuState = state;

  // Reset champion selection when leaving ChampSelect
  if (state !== "ChampSelect" && selectedChampionId) {
    console.log(
      `[LCU] Resetting champion selection (was ${selectedChampionId})`
    );
    selectedChampionId = null;
    // Reset friend lock-in states
    friends.forEach((friend) => {
      friend.isLockedIn = false;
      friend.selectedSkin = null;
    });
    lobbyMembers.clear();
  }
  // When entering ChampSelect, auto-add sharing friends to the party set
  if (state === "ChampSelect") {
    friends.forEach((friend) => {
      if (friend.isSharing) {
        lobbyMembers.add(friend.summonerId);
      }
    });
  }

  io.emit("lcu_state", { state });
  res.json({ state });
  console.log("[LCU] LCU state changed to:", lcuState);
});

// Mock /lol-gameflow/v1/session
app.get("/lol-gameflow/v1/session", (req, res) => {
  console.log("[LCU] GET /lol-gameflow/v1/session");
  // Minimal mock structure, can be expanded as needed
  const session = {
    phase: lcuState,
    gameData: {
      queue: { id: 420 },
      playerChampionSelections: swiftPlaySelections.length
        ? [
          {
            summonerId: user.summonerId,
            championIds: swiftPlaySelections,
          },
        ]
        : [],
      selectedChampions: swiftPlaySelections.map((cid) => ({
        championId: cid,
      })),
    },
    myTeam: [],
    localPlayerCellId: 0,
    actions: [],
  };
  res.json(session);
  console.log("[LCU] Response:", session);
});

// Mock /lol-gameflow/v1/gameflow-phase
app.get("/lol-gameflow/v1/gameflow-phase", (req, res) => {
  console.log("[LCU] GET /lol-gameflow/v1/gameflow-phase");
  res.json(lcuState);
  console.log("[LCU] Response:", lcuState);
});

// Mock /lol-champ-select/v1/session
app.get("/lol-champ-select/v1/session", (req, res) => {
  console.log("[LCU] GET /lol-champ-select/v1/session");

  // Use dynamic champion ID if someone has locked in, otherwise default to 0 (no champion selected)
  let championId = selectedChampionId || 0;
  // In ARAM auto mode, auto-assign a random champ to the local player if not selected yet
  if (aramAutoShare && championId === 0) {
    const randomSkin =
      availableSkins[Math.floor(Math.random() * availableSkins.length)];
    championId = randomSkin.championId;
    console.log(`[TEST][ARAM] Auto-assigned local champion ${championId}`);
  }
  const skinId = championId
    ? availableSkins.find((s) => s.championId === championId)?.skinId ||
    championId * 1000
    : 0;

  // Determine if the pick is completed based on whether a champion is selected
  const isPickCompleted = championId > 0;
  const isPickInProgress = false; // For now, simulate instant completion

  // Example structure from docs
  // Build myTeam from user plus lobby members (friends assumed on same team)
  const myTeam = [
    {
      assignedPosition: "middle",
      cellId: 0,
      championId: championId,
      championPickIntent: 0,
      entitledFeatureType: "",
      selectedSkinId: skinId,
      spell1Id: 4,
      spell2Id: 7,
      summonerId: user.summonerId,
      team: 1,
      wardSkinId: 6,
    },
    ...friends
      .filter((f) => lobbyMembers.has(f.summonerId))
      .map((f, idx) => {
        // In ARAM auto mode, assign a random champ immediately and mark locked
        if (aramAutoShare && !f.isLockedIn) {
          const randomSkin =
            availableSkins[Math.floor(Math.random() * availableSkins.length)];
          f.selectedSkin = randomSkin;
          f.isLockedIn = true;
          // Send skin share immediately
          const conversationId = getConversationId(f.pid);
          const skinShare = {
            from_summoner_id: f.summonerId.toString(),
            from_summoner_name: f.displayName,
            champion_id: randomSkin.championId,
            skin_id: randomSkin.skinId,
            skin_name: randomSkin.skinName,
            chroma_id: null,
            fantome_path: `/${randomSkin.champName}/${randomSkin.skinName
              .toLowerCase()
              .replace(/[^a-z0-9]/g, "_")}.zip`,
            timestamp: Date.now(),
          };
          const partyMessage = { message_type: "skin_share", data: skinShare };
          const messageBody = `OSS:${JSON.stringify(partyMessage)}`;
          const message = {
            id: messageIdCounter++,
            body: messageBody,
            type: "chat",
            fromSummonerId: f.summonerId.toString(),
            timestamp: Date.now(),
          };
          const conversation = Object.values(conversations).find(
            (c) => c.id === conversationId
          );
          conversation.messages.push(message);
          processPartyModeMessage(messageBody, f.summonerId.toString());
          io.emit("chat-message", {
            direction: "received",
            body: messageBody,
            from: f.displayName,
            from_id: f.summonerId.toString(),
            conversationId,
            timestamp: message.timestamp,
          });
        }

        const fChampId =
          f.isLockedIn && f.selectedSkin ? f.selectedSkin.championId : 0;
        const fSkinId =
          f.isLockedIn && f.selectedSkin ? f.selectedSkin.skinId : 0;
        return {
          assignedPosition: "unknown",
          cellId: idx + 1,
          championId: fChampId,
          championPickIntent: 0,
          entitledFeatureType: "",
          selectedSkinId: fSkinId,
          spell1Id: 4,
          spell2Id: 7,
          summonerId: f.summonerId,
          team: 1,
          wardSkinId: 6,
        };
      }),
  ];

  const champSelect = {
    actions: [
      [
        {
          actorCellId: 0,
          championId: championId,
          completed: isPickCompleted,
          id: 1,
          isAllyAction: true,
          isInProgress: isPickInProgress,
          pickTurn: 1,
          type: "pick",
        },
      ],
    ],
    allowBattleBoost: false,
    allowDuplicatePicks: false,
    allowLockedEvents: false,
    allowRerolling: false,
    allowSkinSelection: true,
    benchChampions: [],
    benchEnabled: false,
    boostableSkinCount: 10,
    chatDetails: {
      chatRoomName: "championSelect-12345",
      chatRoomPassword: "password123",
    },
    counter: 30,
    entitledFeatureState: {
      additionalRerolls: 0,
      unlockedSkinIds:
        championId > 0
          ? [championId * 1000, championId * 1000 + 1, skinId]
          : [],
    },
    gameId: 5555555555,
    hasSimultaneousBans: true,
    hasSimultaneousPicks: false,
    isSpectating: false,
    localPlayerCellId: 0,
    lockedEventIndex: -1,
    myTeam,
    phase: isPickCompleted ? "FINALIZATION" : "PLANNING",
    pickOrderSwaps: [],
    recoveryCounter: 30,
    rerollsRemaining: 2,
    skipChampionSelect: false,
    theirTeam: [],
    timer: {
      adjustedTimeLeftInPhase: 25000,
      internalNowInEpochMs: Date.now(),
      isInfinite: false,
      phase: isPickCompleted ? "FINALIZATION" : "PLANNING",
      totalTimeInPhase: 30000,
    },
    trades: [],
  };

  console.log(
    `[LCU] Champion Select Response: championId=${championId}, completed=${isPickCompleted}, phase=${champSelect.phase}`
  );
  res.json(champSelect);
});

// Mock /lol-lobby/v2/lobby to report current party members
app.get("/lol-lobby/v2/lobby", (req, res) => {
  console.log("[LCU] GET /lol-lobby/v2/lobby");
  // Build member list: always include self, and any friends flagged as in lobby
  const members = [
    {
      summonerId: user.summonerId,
      puuid: user.puuid,
      displayName: user.displayName,
      gameName: user.gameName,
      gameTag: user.gameTag,
    },
    ...friends
      .filter((f) => lobbyMembers.has(f.summonerId))
      .map((f) => ({
        summonerId: f.summonerId,
        puuid: f.puuid,
        displayName: f.displayName,
        gameName: f.gameName,
        gameTag: f.gameTag,
      })),
  ];
  const lobby = {
    partyType: "open",
    gameConfig: { queueId: aramAutoShare ? 450 : 420 },
    members,
    localMember: {
      playerSlots: swiftPlaySelections.map((cid) => ({ championId: cid })),
    },
  };
  res.json(lobby);
  console.log(
    "[LCU] Response members:",
    members.map((m) => m.displayName)
  );
});

// TEST: toggle friend in/out of current lobby (party)
app.post("/test/toggle-friend-in-lobby", (req, res) => {
  const { friendIndex = 0 } = req.body;
  const friend = friends[friendIndex];
  if (!friend) {
    return res.status(400).json({ error: "Friend not found" });
  }
  if (lobbyMembers.has(friend.summonerId)) {
    lobbyMembers.delete(friend.summonerId);
  } else {
    lobbyMembers.add(friend.summonerId);
  }
  const inLobby = lobbyMembers.has(friend.summonerId);
  console.log(
    `[TEST] ${friend.displayName} is now ${inLobby ? "IN" : "OUT OF"} lobby`
  );
  res.json({ success: true, inLobby });
});

// Serve dashboard
const path = require("path");
app.get("/dashboard", (req, res) => {
  res.sendFile(path.join(__dirname, "dashboard.html"));
});

const PORT = 56174;
server.listen(PORT, () => {
  console.log(`Mock LCU server running on https://localhost:${PORT}`);
  console.log(`Dashboard available at: https://localhost:${PORT}/dashboard`);
  console.log(`\nEndpoints implemented:`);
  console.log(`- GET  /lol-chat/v1/friends`);
  console.log(`- GET  /lol-chat/v1/conversations`);
  console.log(`- GET  /lol-chat/v1/conversations/:id/messages`);
  console.log(`- POST /lol-chat/v1/conversations/:id/messages`);
  console.log(`- POST /lol-chat/v1/conversations`);
  console.log(`- GET  /lol-summoner/v1/current-summoner`);
  console.log(`- GET  /lol-gameflow/v1/session`);
  console.log(`- GET  /lol-gameflow/v1/gameflow-phase`);
  console.log(`- GET  /lol-champ-select/v1/session`);
  console.log(`\nParty Mode Test endpoints:`);
  console.log(`- POST /test/toggle-friend-sharing`);
  console.log(`- POST /test/friend-lock-skin`);
  console.log(`- POST /test/local-player-lock-skin`);
  console.log(`- POST /test/set-selected-champion`);
  console.log(`- GET  /test/party-state`);
  console.log(`- POST /test/clear-party-state`);
  console.log(`- POST /test/swift-play/select`);
  console.log(`- POST /test/swift-play/clear`);
  console.log(`- POST /test/aram/enable`);
  console.log(`- POST /test/aram/disable`);
  console.log(`- POST /test/friends/sharing`);
  console.log(`- POST /test/friends/lobby`);
});

// Swift Play: set up to two lobby selections
app.post("/test/swift-play/select", (req, res) => {
  const { championIds = [] } = req.body;
  swiftPlaySelections = Array.from(new Set(championIds))
    .slice(0, 2)
    .map((n) => Number(n));
  console.log("[TEST] Swift Play selections:", swiftPlaySelections);
  res.json({ success: true, selections: swiftPlaySelections });
});

app.post("/test/swift-play/clear", (req, res) => {
  swiftPlaySelections = [];
  console.log("[TEST] Swift Play selections cleared");
  res.json({ success: true });
});

// ARAM: enable/disable auto-assign + auto-share for friends
app.post("/test/aram/enable", (req, res) => {
  aramAutoShare = true;
  console.log("[TEST] ARAM auto-share ENABLED");
  res.json({ success: true });
});

app.post("/test/aram/disable", (req, res) => {
  aramAutoShare = false;
  console.log("[TEST] ARAM auto-share DISABLED");
  res.json({ success: true });
});
