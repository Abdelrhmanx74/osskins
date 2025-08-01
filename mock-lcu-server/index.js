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
  summonerId: "148331403",
  displayName: "rogolax",
  pid: "5db00f0b-8d04-5fe9-8fdb-5f768cbced5b@eu1.pvp.net",
};

const friends = [
  {
    summonerId: "129649534",
    summoner_name: "friend1",
    displayName: "friend1",
    is_online: true,
    availability: "online",
    puuid: "puuid-friend1",
    pid: "0e624b10-3d3e-5222-ac66-c0dd162d83e8@eu1.pvp.net",
  },
  {
    summonerId: "3458743863674208",
    summoner_name: "sisi",
    displayName: "sisi",
    is_online: true,
    availability: "online",
    puuid: "puuid-sisi",
    pid: "d56d6f6e-5540-5a46-9724-116f99cf98f0@eu1.pvp.net",
  },
];

// Party mode state
let pairingRequests = [];
let sentRequests = [];
let pairedFriends = [];
let lcuState = "Lobby"; // Lobby, ChampSelect, InProgress, Reconnect
let champSelectSession = {};

// Socket.IO connection
io.on("connection", (socket) => {
  console.log("Client connected:", socket.id);
  socket.on("disconnect", () => {
    console.log("Client disconnected:", socket.id);
  });
});

// --- Endpoints ---
// Get friends list
app.get("/lol-chat/v1/friends", (req, res) => {
  console.log("[API] GET /lol-chat/v1/friends");
  // Map mock data to expected frontend structure
  const mappedFriends = friends.map((f) => ({
    summoner_id: f.summonerId,
    summoner_name: f.summoner_name,
    display_name: f.displayName,
    is_online: f.is_online,
    availability: f.availability,
    puuid: f.puuid,
    pid: f.pid,
  }));
  res.json(mappedFriends);
  console.log("[API] Response:", mappedFriends);
});

// Send pairing request (simulate friend -> user)
app.post("/party-mode/v1/pairing-request", (req, res) => {
  const { from_summoner_id, from_summoner_name, request_id, timestamp } =
    req.body;
  console.log("[API] POST /party-mode/v1/pairing-request", req.body);
  pairingRequests.push({
    from_summoner_id,
    from_summoner_name,
    request_id,
    timestamp,
  });
  // Emit event to frontend (open accept modal)
  io.emit("pairing_request", {
    from_summoner_id,
    from_summoner_name,
    request_id,
    timestamp,
  });
  // Vocal feedback
  say.speak(`${from_summoner_name} sent you a sync request.`);
  res.json({ ok: true });
});

// Respond to pairing request (accept/decline)
app.post("/party-mode/v1/pairing-response", (req, res) => {
  const {
    accepted,
    from_summoner_id,
    from_summoner_name,
    request_id,
    timestamp,
  } = req.body;
  console.log("[API] POST /party-mode/v1/pairing-response", req.body);
  // Remove from pending requests
  pairingRequests = pairingRequests.filter((r) => r.request_id !== request_id);
  // If accepted, add to paired friends with correct structure
  if (accepted) {
    pairedFriends.push({
      summonerId: from_summoner_id,
      summoner_name: from_summoner_name,
      displayName: from_summoner_name,
      paired_at: timestamp || Date.now(),
    });
    io.emit("pairing_response", {
      accepted,
      from_summoner_id,
      from_summoner_name,
      request_id,
      timestamp,
    });
    say.speak(`You accepted the sync request from ${from_summoner_name}.`);
  } else {
    io.emit("pairing_response", {
      accepted,
      from_summoner_id,
      from_summoner_name,
      request_id,
      timestamp,
    });
    say.speak(`You declined the sync request from ${from_summoner_name}.`);
  }
  res.json({ ok: true });
});

// Get paired friends
app.get("/party-mode/v1/paired-friends", (req, res) => {
  console.log("[API] GET /party-mode/v1/paired-friends");
  // Map to expected frontend structure
  const mappedPaired = pairedFriends.map((f) => ({
    summoner_id: f.summonerId,
    summoner_name: f.summoner_name || f.displayName,
    display_name: f.displayName,
    paired_at: f.paired_at || Date.now(),
  }));
  res.json(mappedPaired);
  console.log("[API] Response:", mappedPaired);
});

// Get sent requests
app.get("/party-mode/v1/sent-requests", (req, res) => {
  console.log("[API] GET /party-mode/v1/sent-requests");
  res.json(sentRequests);
  console.log("[API] Response:", sentRequests);
});

// Get pending pairing requests
app.get("/party-mode/v1/pairing-requests", (req, res) => {
  console.log("[API] GET /party-mode/v1/pairing-requests");
  res.json(pairingRequests);
  console.log("[API] Response:", pairingRequests);
});

// Party mode settings (mock)
let partyModeSettings = { enabled: true };
app.get("/party-mode/v1/settings", (req, res) => {
  console.log("[API] GET /party-mode/v1/settings");
  res.json(partyModeSettings);
  console.log("[API] Response:", partyModeSettings);
});
app.post("/party-mode/v1/settings", (req, res) => {
  console.log("[API] POST /party-mode/v1/settings", req.body);
  partyModeSettings = { ...partyModeSettings, ...req.body };
  res.json(partyModeSettings);
  console.log("[API] Updated settings:", partyModeSettings);
});

// Champ select session (mock)
app.get("/champ-select/v1/session", (req, res) => {
  console.log("[API] GET /champ-select/v1/session");
  res.json(champSelectSession);
  console.log("[API] Response:", champSelectSession);
});
app.post("/champ-select/v1/session", (req, res) => {
  console.log("[API] POST /champ-select/v1/session", req.body);
  champSelectSession = { ...champSelectSession, ...req.body };
  io.emit("champ_select_session", champSelectSession);
  res.json(champSelectSession);
  console.log("[API] Updated champ select session:", champSelectSession);
});

// Skin share (mock)
app.post("/party-mode/v1/skin-share", (req, res) => {
  const data = req.body;
  console.log("[API] POST /party-mode/v1/skin-share", data);
  io.emit("skin_share", data);
  res.json({ ok: true });
  console.log("[API] Skin share event emitted:", data);
});

// LCU state transitions
app.get("/lcu/v1/state", (req, res) => {
  console.log("[API] GET /lcu/v1/state");
  res.json({ state: lcuState });
  console.log("[API] Response:", lcuState);
});
app.post("/lcu/v1/state", (req, res) => {
  const { state } = req.body;
  console.log("[API] POST /lcu/v1/state", req.body);
  lcuState = state;
  io.emit("lcu_state", { state });
  res.json({ state });
  console.log("[API] LCU state changed to:", lcuState);
});

// Mock /lol-gameflow/v1/session
app.get("/lol-gameflow/v1/session", (req, res) => {
  console.log("[API] GET /lol-gameflow/v1/session");
  // Minimal mock structure, can be expanded as needed
  const session = {
    phase: lcuState,
    gameData: {
      queue: { id: 420 },
      playerChampionSelections: [],
      selectedChampions: [],
    },
    myTeam: [],
    localPlayerCellId: 0,
    actions: [],
  };
  res.json(session);
  console.log("[API] Response:", session);
});

// Mock /lol-gameflow/v1/gameflow-phase
app.get("/lol-gameflow/v1/gameflow-phase", (req, res) => {
  console.log("[API] GET /lol-gameflow/v1/gameflow-phase");
  res.json(lcuState);
  console.log("[API] Response:", lcuState);
});

// Mock /lol-champ-select/v1/session
app.get("/lol-champ-select/v1/session", (req, res) => {
  console.log("[API] GET /lol-champ-select/v1/session");
  // Example structure from docs
  const champSelect = {
    actions: [
      [
        {
          actorCellId: 0,
          championId: 81,
          completed: true,
          id: 1,
          isAllyAction: true,
          isInProgress: false,
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
      unlockedSkinIds: [81000, 81001, 81021],
    },
    gameId: 5555555555,
    hasSimultaneousBans: true,
    hasSimultaneousPicks: false,
    isSpectating: false,
    localPlayerCellId: 0,
    lockedEventIndex: -1,
    myTeam: [
      {
        assignedPosition: "middle",
        cellId: 0,
        championId: 81,
        championPickIntent: 81,
        entitledFeatureType: "",
        selectedSkinId: 81021,
        spell1Id: 4,
        spell2Id: 7,
        summonerId: 123456789,
        team: 1,
        wardSkinId: 6,
      },
    ],
    phase: "FINALIZATION",
    pickOrderSwaps: [],
    recoveryCounter: 30,
    rerollsRemaining: 2,
    skipChampionSelect: false,
    theirTeam: [],
    timer: {
      adjustedTimeLeftInPhase: 25000,
      internalNowInEpochMs: Date.now(),
      isInfinite: false,
      phase: "FINALIZATION",
      totalTimeInPhase: 30000,
    },
    trades: [],
  };
  res.json(champSelect);
  console.log("[API] Response:", champSelect);
});

// Serve dashboard
const path = require("path");
app.use("/dashboard", express.static(path.join(__dirname, "dashboard.html")));

const PORT = 56174;
server.listen(PORT, () => {
  console.log(`Mock LCU server running on https://localhost:${PORT}`);
});
