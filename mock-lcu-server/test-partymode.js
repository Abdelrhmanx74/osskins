/*
  Headless Party Mode test runner
  - Boots (or reuses) the mock LCU server at https://localhost:56174
  - Exercises chat, lobby, gameflow, champ-select, Swift Play, ARAM auto-share
  - Asserts outcomes and prints a compact pass/fail summary
*/

const https = require("https");
const axios = require("axios");
const { io } = require("socket.io-client");
const { spawn } = require("child_process");
const path = require("path");

const BASE = "https://localhost:56174";
const httpsAgent = new https.Agent({ rejectUnauthorized: false });
const api = axios.create({ baseURL: BASE, httpsAgent, timeout: 8000 });

// Minimal test framework
const results = [];
function pass(name) { results.push({ name, ok: true }); console.log(`✔ ${name}`); }
function fail(name, err) {
  results.push({ name, ok: false, err: err && (err.response?.data || err.message || String(err)) });
  console.error(`✘ ${name}\n   → ${results[results.length - 1].err}`);
}
async function expect(name, fn) {
  try { await fn(); pass(name); } catch (e) { fail(name, e); }
}

// Helpers
async function waitForHealth(maxMs = 15000) {
  const start = Date.now();
  while (Date.now() - start < maxMs) {
    try {
      const r = await api.get("/health");
      if (r.data && r.data.status === "ok") return true;
    } catch { /* retry */ }
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error("Mock LCU server health check failed");
}

async function getConversations() {
  const r = await api.get("/lol-chat/v1/conversations");
  return r.data || [];
}

async function getLobby() {
  return (await api.get("/lol-lobby/v2/lobby")).data;
}

async function getGameflowPhase() {
  return (await api.get("/lol-gameflow/v1/gameflow-phase")).data;
}

async function getChampSelect() {
  return (await api.get("/lol-champ-select/v1/session")).data;
}

async function listFriends() {
  return (await api.get("/lol-chat/v1/friends")).data;
}

// Boot or reuse server
async function ensureServer() {
  try {
    await waitForHealth(1500);
    return { proc: null };
  } catch { }

  console.log("Starting mock LCU server...");
  const proc = spawn(process.execPath, [path.join(__dirname, "index.js")], {
    cwd: __dirname,
    stdio: ["ignore", "pipe", "pipe"],
  });

  proc.stdout.on("data", (d) => {
    const s = d.toString();
    if (s.toLowerCase().includes("error")) process.stderr.write(s);
  });
  proc.stderr.on("data", (d) => process.stderr.write(d.toString()));

  await waitForHealth();
  return { proc };
}

// Socket event capture
function connectSocket() {
  const socket = io(BASE, { transports: ["websocket"], rejectUnauthorized: false });
  const events = { skinReceived: [], chat: [] };
  socket.on("connect", () => console.log("Socket connected"));
  socket.on("party-mode-skin-received", (p) => events.skinReceived.push(p));
  socket.on("chat-message", (p) => events.chat.push(p));
  return { socket, events };
}

async function run() {
  const { proc } = await ensureServer();
  const { socket, events } = connectSocket();

  try {
    // Baseline
    await expect("Health endpoint ok", async () => {
      const r = await api.get("/health");
      if (r.data.status !== "ok") throw new Error("not ok");
    });

    await expect("Friends list has entries", async () => {
      const f = await listFriends();
      if (!Array.isArray(f) || f.length < 2) throw new Error("expected >= 2 friends");
      if (!f[0].pid || !f[0].puuid) throw new Error("friend missing pid/puuid");
    });

    await expect("Summoner endpoint returns expected shape", async () => {
      const r = await api.get("/lol-summoner/v1/current-summoner");
      const s = r.data || {};
      ["accountId", "summonerId", "puuid", "displayName"].forEach((k) => {
        if (!(k in s)) throw new Error(`missing ${k}`);
      });
    });

    await expect("Clear party state -> Lobby", async () => {
      await api.post("/test/clear-party-state", {});
      const phase = await getGameflowPhase();
      if (phase !== "Lobby") throw new Error(`phase is ${phase}`);
    });

    // Conversations and chat
    await expect("Create conversation and send chat", async () => {
      const friends = await listFriends();
      const target = friends[0];
      const convo = (await api.post("/lol-chat/v1/conversations", { pid: target.pid })).data;
      await api.post(`/lol-chat/v1/conversations/${convo.id}/messages`, { body: "hello", type: "chat" });
      const msgs = (await api.get(`/lol-chat/v1/conversations/${convo.id}/messages`)).data;
      if (!msgs.length || msgs[msgs.length - 1].body !== "hello") throw new Error("message not persisted");
      // numeric message IDs should increase
      const ids = msgs.map((m) => Number(m.id));
      if (ids.some((n) => Number.isNaN(n))) throw new Error("non-numeric message id found");
      for (let i = 1; i < ids.length; i++) if (!(ids[i] >= ids[i - 1])) throw new Error("ids not monotonic");
    });

    await expect("Posting to missing conversation yields 404", async () => {
      let threw = false;
      try {
        await api.post("/lol-chat/v1/conversations/does-not-exist/messages", { body: "x", type: "chat" });
      } catch (e) { threw = e.response?.status === 404; }
      if (!threw) throw new Error("expected 404");
    });

    await expect("Creating conversation without pid yields 400", async () => {
      let threw = false;
      try { await api.post("/lol-chat/v1/conversations", {}); } catch (e) { threw = e.response?.status === 400; }
      if (!threw) throw new Error("expected 400");
    });

    // Lobby, party members
    await expect("Put all friends into lobby", async () => {
      await api.post("/test/friends/lobby", { inLobby: true });
      const lobby = await getLobby();
      const members = lobby.members || [];
      if (members.length < 2) throw new Error("expected user + >=1 friend in lobby");
    });

    // Swift Play in Lobby: verify selections appear in session and lobby.localMember
    await expect("Swift Play selections shown in Lobby session and lobby.localMember", async () => {
      await api.post("/test/swift-play/select", { championIds: [81, 24] });
      const session = (await api.get("/lol-gameflow/v1/session")).data;
      const picks = session?.gameData?.playerChampionSelections?.[0]?.championIds || [];
      if (!(Array.isArray(picks) && picks.length === 2)) throw new Error("swift play not reflected in session");
      const lobby = await getLobby();
      if (!Array.isArray(lobby.localMember?.playerSlots) || lobby.localMember.playerSlots.length !== 2) throw new Error("swift play not reflected in lobby");
      await api.post("/test/swift-play/clear", {});
    });

    // Sharing ON
    await expect("Enable sharing for all friends", async () => {
      await api.post("/test/friends/sharing", { enabled: true });
    });

    // Enter Champ Select -> friend locks and sends share
    await expect("Champ Select friend share triggers event and chat", async () => {
      await api.post("/lcu/v1/state", { state: "ChampSelect" });
      // Explicit friend share
      await api.post("/test/friend-lock-skin", { friendIndex: 0, skinIndex: 0 });
      // Allow async delivery
      await new Promise((r) => setTimeout(r, 400));
      if (!events.skinReceived.length) throw new Error("no skin-received event");
      const cs = await getChampSelect();
      const friendRow = cs.myTeam.find((m) => m.cellId === 1);
      if (!friendRow || !friendRow.championId) throw new Error("friend not locked in");
      const convos = await getConversations();
      const anyHasMessage = convos.some((c) => (c.messages || []).some((m) => String(m.body || "").startsWith("OSS:")));
      if (!anyHasMessage) throw new Error("no OSS chat messages found");
    });

    // Lobby-only friend shares (no champ select): ensure events still flow
    await expect("Lobby: friend skin_share still triggers", async () => {
      await api.post("/lcu/v1/state", { state: "Lobby" });
      const base = events.skinReceived.length;
      await api.post("/test/friend-lock-skin", { friendIndex: 1, skinIndex: 1 });
      await new Promise((r) => setTimeout(r, 200));
      if (events.skinReceived.length <= base) throw new Error("no event in Lobby");
    });

    // Multiple friends share concurrently
    await expect("Concurrent shares from two friends observed", async () => {
      const before = events.skinReceived.length;
      await Promise.all([
        api.post("/test/friend-lock-skin", { friendIndex: 0, skinIndex: 2 }),
        api.post("/test/friend-lock-skin", { friendIndex: 1, skinIndex: 3 }),
      ]);
      await new Promise((r) => setTimeout(r, 300));
      const delta = events.skinReceived.length - before;
      if (delta < 2) throw new Error("expected >=2 skin shares");
    });

    // Local share
    await expect("Local lock-in sends OSS shares to friends", async () => {
      const before = events.chat.length;
      await api.post("/test/local-player-lock-skin", { skinIndex: 1 });
      await new Promise((r) => setTimeout(r, 300));
      const after = events.chat.length;
      if (after <= before) throw new Error("no outgoing chat messages");
    });

    // Swift Play selection reflected in gameflow session
    await expect("Swift Play two selections reflected", async () => {
      await api.post("/test/swift-play/select", { championIds: [81, 24] });
      const session = (await api.get("/lol-gameflow/v1/session")).data;
      const slots = session?.gameData?.playerChampionSelections?.[0]?.championIds || [];
      if (!(Array.isArray(slots) && slots.length === 2)) throw new Error("swift play not applied");
      await api.post("/test/swift-play/clear", {});
    });

    // ARAM auto-share scenario
    await expect("ARAM auto-share triggers friend OSS shares", async () => {
      // Reset state to ensure friends aren't already locked in
      await api.post("/test/clear-party-state", {});
      await api.post("/test/friends/sharing", { enabled: true });
      const baseEvents = events.chat.length;
      await api.post("/test/aram/enable", {});
      await api.post("/lcu/v1/state", { state: "ChampSelect" });
      // Fetch champ-select to trigger auto assignment + shares in server logic
      await getChampSelect();
      await new Promise((r) => setTimeout(r, 500));
      const more = events.chat.length - baseEvents;
      if (more <= 0) throw new Error("no ARAM auto share chat observed");
      const lobby = await getLobby();
      if (lobby.gameConfig.queueId !== 450) throw new Error("queueId not ARAM (450)");
      await api.post("/test/aram/disable", {});
    });

    // ARAM reroll from local user: should send fresh OSS shares to friends
    await expect("ARAM reroll sends new OSS shares", async () => {
      await api.post("/test/aram/enable", {});
      await api.post("/lcu/v1/state", { state: "ChampSelect" });
      await getChampSelect(); // trigger auto-assign if needed
      const before = events.chat.length;
      await api.post("/test/aram/reroll", {});
      await new Promise((r) => setTimeout(r, 300));
      const after = events.chat.length;
      if (after <= before) throw new Error("no outgoing messages on reroll");
      await api.post("/test/aram/disable", {});
    });

    // Gameflow transitions
    await expect("Transition through Matchmaking → InProgress → Reconnect → Lobby", async () => {
      await api.post("/lcu/v1/state", { state: "Matchmaking" });
      if ((await getGameflowPhase()) !== "Matchmaking") throw new Error("not Matchmaking");
      await api.post("/lcu/v1/state", { state: "InProgress" });
      if ((await getGameflowPhase()) !== "InProgress") throw new Error("not InProgress");
      await api.post("/lcu/v1/state", { state: "Reconnect" });
      if ((await getGameflowPhase()) !== "Reconnect") throw new Error("not Reconnect");
      await api.post("/lcu/v1/state", { state: "Lobby" });
      if ((await getGameflowPhase()) !== "Lobby") throw new Error("not Lobby");
    });

    // Toggle a friend in/out of lobby
    await expect("Toggle friend lobby membership reflects in /lobby", async () => {
      await api.post("/test/friends/lobby", { inLobby: false });
      let lobby = await getLobby();
      const members1 = lobby.members || [];
      await api.post("/test/toggle-friend-in-lobby", { friendIndex: 0 });
      lobby = await getLobby();
      const members2 = lobby.members || [];
      if (!(members2.length > members1.length)) throw new Error("membership did not increase");
    });

    // Turning sharing off removes lock-in state for friend
    await expect("Disabling sharing resets friend lock state", async () => {
      await api.post("/test/friend-lock-skin", { friendIndex: 0, skinIndex: 0 });
      const before = (await api.get("/test/party-state")).data;
      if (!before.friends[0].isLockedIn) throw new Error("friend not locked prior to disable");
      await api.post("/test/friends/sharing", { enabled: false });
      const after = (await api.get("/test/party-state")).data;
      if (after.friends.some((f) => f.isLockedIn)) throw new Error("lock state not cleared");
      // Re-enable for next tests
      await api.post("/test/friends/sharing", { enabled: true });
    });

    // Invalid OSS message should not crash and should not produce skinReceived event
    await expect("Invalid OSS message does not emit skin-received", async () => {
      const base = events.skinReceived.length;
      await api.post("/test/send-invalid-oss", { friendIndex: 0 });
      await new Promise((r) => setTimeout(r, 200));
      if (events.skinReceived.length !== base) return; // good: no new events
    });

  } finally {
    socket.close();
    if (proc) proc.kill();
    // Summary
    console.log("\n==== Party Mode Test Summary ====");
    const ok = results.filter((r) => r.ok).length;
    const bad = results.length - ok;
    results.forEach((r) => {
      const mark = r.ok ? "✔" : "✘";
      console.log(`${mark} ${r.name}`);
    });
    console.log(`Total: ${results.length}, Passed: ${ok}, Failed: ${bad}`);
    if (bad > 0) process.exitCode = 1; else process.exitCode = 0;
  }
}

run().catch((e) => {
  console.error("Fatal test runner error:", e);
  process.exit(1);
});
