import { useCallback, useEffect, useRef } from "react";
import { useGameStore, PartyMember } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import pako from "pako";
import { listen } from "@tauri-apps/api/event";

interface SkinSyncData {
  userSkins: {
    championId: number;
    skinId: number;
    chromaId: number | null;
    fantome: string | null;
  }[];
  requestType: "request" | "accept" | "reject";
  version: 1;
}

// Move these above all useEffect/useCallback
const compressSkinData = (data: SkinSyncData): string => {
  try {
    const jsonString = JSON.stringify(data);
    const compressed = pako.deflate(jsonString);
    return btoa(
      String.fromCharCode.apply(null, compressed as unknown as number[])
    );
  } catch (error) {
    console.error("Failed to compress skin data:", error);
    throw new Error("Failed to compress skin data");
  }
};

// Helper to check if a string is valid base64
function isValidBase64(str: string): boolean {
  // Remove whitespace and check length
  if (!str || str.length % 4 !== 0) return false;
  // Only allow base64 chars
  return /^[A-Za-z0-9+/=]+$/.test(str);
}

// Make the base64 validation and decoding more robust
function safeBase64ToBytes(str: string): Uint8Array | null {
  try {
    // If the string doesn't look like base64 at all, return null early
    if (!str || typeof str !== "string") return null;

    // Remove all whitespace, linebreaks and other non-base64 characters
    let cleaned = str.replace(/[^A-Za-z0-9+/=]/g, "");

    // Add padding if needed
    while (cleaned.length % 4 !== 0) cleaned += "=";

    try {
      const binary = atob(cleaned);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
      return bytes;
    } catch {
      return null;
    }
  } catch {
    return null;
  }
}

const decompressSkinData = (base64Data: string): SkinSyncData => {
  try {
    const bytes = safeBase64ToBytes(base64Data);
    if (!bytes) throw new Error("Invalid base64 string");

    try {
      const decompressed = pako.inflate(bytes);
      const jsonString = new TextDecoder().decode(decompressed);
      return JSON.parse(jsonString);
    } catch (error) {
      console.error("Failed to decompress skin data:", error);
      throw new Error("Failed to decompress skin data");
    }
  } catch (error) {
    // Create a more detailed error message for debugging
    console.error(
      `Base64 validation failed for: ${base64Data?.substring(
        0,
        20
      )}... (length: ${base64Data?.length || 0})`
    );
    throw error;
  }
};

/**
 * Hook for handling skin synchronization between friends
 */
export function useSkinSync() {
  const {
    selectedSkins,
    partyMembers,
    pendingSyncRequest,
    addPartyMember,
    removePartyMember,
    updatePartyMemberSkins,
    setPendingSyncRequest,
    leaguePath,
  } = useGameStore();

  // Track if we've already checked for pending sync requests this session
  const checkedPendingRef = useRef(false);

  // Track when we last checked for pending sync requests
  const lastCheckTimeRef = useRef(0);

  // Key function to check for pending sync requests in message history
  const checkPendingSyncRequests = useCallback(
    async (force = false) => {
      if (!leaguePath) return;

      // Only check every 30 seconds unless forced
      const now = Date.now();
      if (!force && now - lastCheckTimeRef.current < 30000) return;
      lastCheckTimeRef.current = now;

      console.log("Checking for pending skin sync requests...");

      try {
        // Get friends list from LCU
        const friends = await invoke<any[]>("get_lcu_friends", { leaguePath });
        console.log(
          `Found ${friends.length} friends to check for sync requests`
        );

        for (const friend of friends) {
          try {
            // Fetch recent messages from this friend
            console.log(`Checking messages from ${friend.name} (${friend.id})`);
            const messages = await invoke<any>("get_lcu_messages", {
              leaguePath,
              friendId: friend.id,
            });

            if (Array.isArray(messages)) {
              console.log(
                `Found ${messages.length} messages with ${friend.name}`
              );

              // Look for the latest OSS-SKIN-SYNC request that hasn't been accepted/rejected
              for (let i = messages.length - 1; i >= 0; i--) {
                const msg = messages[i];
                if (
                  typeof msg.body === "string" &&
                  msg.body.includes("[OSS-SKIN-SYNC]")
                ) {
                  // Only show if it's a request (not accept/reject)
                  const match = msg.body.match(
                    /\[OSS-SKIN-SYNC\](.*?)\[\/OSS-SKIN-SYNC\]/
                  );
                  if (match && match[1]) {
                    try {
                      const decoded = decompressSkinData(match[1]);
                      if (decoded.requestType === "request") {
                        console.log(`Found sync request from ${friend.name}`);

                        // Only show if we haven't already accepted/rejected this request
                        if (
                          !pendingSyncRequest ||
                          pendingSyncRequest.memberId !== friend.id
                        ) {
                          setPendingSyncRequest({
                            memberId: friend.id,
                            memberName: friend.name,
                            data: match[1],
                          });
                          toast.info(
                            `You have a pending sync request from ${friend.name}`
                          );
                          return; // Stop after finding the first request
                        }
                      }
                    } catch (err) {
                      console.error("Failed to decode sync request:", err);
                      // If base64 or decompression fails, skip this message
                      continue;
                    }
                  }
                }
              }
            }
          } catch (err) {
            console.error(`Error checking messages with ${friend.name}:`, err);
            continue; // Try the next friend if this one fails
          }
        }
        console.log("No pending skin sync requests found");
      } catch (err) {
        console.error("Error checking for sync requests:", err);
        // Ignore errors (e.g. if League isn't running)
      }
    },
    [leaguePath, pendingSyncRequest, setPendingSyncRequest]
  );

  // Check for pending sync requests on startup and when leaguePath changes
  useEffect(() => {
    if (leaguePath) {
      checkPendingSyncRequests(true);
    }
  }, [leaguePath, checkPendingSyncRequests]);

  // Set up a listener for chat messages to detect skin sync requests
  useEffect(() => {
    if (!leaguePath) return;

    // Poll for new messages periodically
    const intervalId = setInterval(() => {
      pollForSyncMessages();
    }, 10000); // Every 10 seconds

    return () => {
      clearInterval(intervalId);
    };
  }, [leaguePath]);

  // Check for pending sync requests in message history when Party Mode dialog is opened or user comes online
  useEffect(() => {
    if (!leaguePath || checkedPendingRef.current) return;
    checkedPendingRef.current = true;
    (async () => {
      try {
        // Get friends list from LCU
        const friends = await invoke<any[]>("get_lcu_friends", { leaguePath });
        for (const friend of friends) {
          // Fetch recent messages from this friend
          const messages = await invoke<any>("get_lcu_messages", {
            leaguePath,
            friendId: friend.id,
          });
          if (Array.isArray(messages)) {
            // Look for the latest OSS-SKIN-SYNC request that hasn't been accepted/rejected
            for (let i = messages.length - 1; i >= 0; i--) {
              const msg = messages[i];
              if (
                typeof msg.body === "string" &&
                msg.body.includes("[OSS-SKIN-SYNC]")
              ) {
                // Only show if it's a request (not accept/reject)
                const match = msg.body.match(
                  /\[OSS-SKIN-SYNC\](.*?)\[\/OSS-SKIN-SYNC\]/
                );
                if (match && match[1]) {
                  try {
                    const decoded = decompressSkinData(match[1]);
                    if (decoded.requestType === "request") {
                      // Only show if we haven't already accepted/rejected this request
                      if (
                        !pendingSyncRequest ||
                        pendingSyncRequest.memberId !== friend.id
                      ) {
                        setPendingSyncRequest({
                          memberId: friend.id,
                          memberName: friend.name,
                          data: match[1],
                        });
                        toast.info(
                          `You have a pending sync request from ${friend.name}`
                        );
                        return;
                      }
                    }
                  } catch (err) {
                    // If base64 or decompression fails, skip this message
                    continue;
                  }
                }
              }
            }
          }
        }
      } catch (err) {
        // Ignore errors (e.g. if League isn't running)
      }
    })();
  }, [
    leaguePath,
    setPendingSyncRequest,
    pendingSyncRequest,
    decompressSkinData,
  ]);

  // Poll for messages that might contain sync requests
  const pollForSyncMessages = useCallback(async () => {
    if (!leaguePath || partyMembers.length >= 4) return;

    try {
      // In a real implementation, this would check recent messages from friends
      // For now, we'll use a mock implementation
      // Uncommenting this would create a random mock sync request for testing
      /*
      if (Math.random() < 0.2 && !pendingSyncRequest) { // 20% chance
        const mockFriendId = `friend_${Date.now()}`;
        const mockFriendName = "TestFriend";
        
        const mockSkins = [
          {
            championId: 266,
            skinId: 266021,
            chromaId: 266029,
            fantome: "aatrox/lunar_eclipse_aatrox_chroma_266029.fantome",
          }
        ];
        
        const syncData: SkinSyncData = {
          userSkins: mockSkins,
          requestType: "request",
          version: 1
        };
        
        const encodedData = compressSkinData(syncData);
        
        setPendingSyncRequest({
          memberId: mockFriendId,
          memberName: mockFriendName,
          data: encodedData
        });
        
        toast.info(`Received sync request from ${mockFriendName}`);
      }
      */
    } catch (error) {
      console.error("Error polling for sync messages:", error);
    }
  }, [leaguePath, partyMembers, pendingSyncRequest]);

  // Send a sync request to a friend
  const sendSyncRequest = useCallback(
    async (friendId: string, friendName: string) => {
      try {
        // Check if we've reached the maximum party size (5 including the user)
        if (partyMembers.length >= 4) {
          toast.error("Party can have a maximum of 5 members");
          return Promise.reject("Party size limit reached");
        }

        // Make sure we have a league path
        if (!leaguePath) {
          toast.error(
            "League path not set. Please configure your League of Legends path in settings."
          );
          return Promise.reject("League path not set");
        }

        // Convert selected skins Map to array
        const skinArray = Array.from(selectedSkins.values()).map((skin) => ({
          championId: skin.championId,
          skinId: skin.skinId,
          chromaId: skin.chromaId || null,
          fantome: skin.fantome || null,
        }));

        // Prepare data for sync
        const syncData: SkinSyncData = {
          userSkins: skinArray,
          requestType: "request",
          version: 1,
        };

        // Compress and encode data
        const encodedData = compressSkinData(syncData);

        // Generate the special message format that contains the sync code
        const syncMessage = `[OSS-SKIN-SYNC]${encodedData}[/OSS-SKIN-SYNC]`;

        // Send message via LCU API
        try {
          await invoke("send_lcu_message", {
            leaguePath,
            friendId,
            message: syncMessage,
          });

          // Add friend to party
          addPartyMember({
            id: friendId,
            name: friendName,
            availability: "online",
            skins: new Map(),
          });

          toast.success(`Sync request sent to ${friendName}`);
        } catch (error) {
          console.error("Failed to send LCU message:", error);
          toast.error(
            `Could not send message to ${friendName}. Make sure League of Legends is running.`
          );
          return Promise.reject(error);
        }

        return Promise.resolve();
      } catch (error) {
        console.error("Failed to send sync request:", error);
        return Promise.reject(error);
      }
    },
    [selectedSkins, partyMembers, compressSkinData, addPartyMember, leaguePath]
  );

  // Accept a sync request from a friend
  const acceptSync = useCallback(
    async (friendId: string, friendName: string) => {
      try {
        if (!pendingSyncRequest) {
          return;
        }

        // Decode the received data
        const decodedData = decompressSkinData(pendingSyncRequest.data);
        console.log("Decoded skin data:", decodedData);

        // Create a new map of skins from the friend
        const friendSkins = new Map();
        decodedData.userSkins.forEach((skin) => {
          friendSkins.set(skin.championId, {
            championId: skin.championId,
            skinId: skin.skinId,
            chromaId: skin.chromaId || undefined,
            fantome: skin.fantome || undefined,
          });
        });

        // Add the friend to our party with their skins
        addPartyMember({
          id: friendId,
          name: friendName,
          availability: "online",
          skins: friendSkins,
        });

        // Clear the pending request
        setPendingSyncRequest(null);

        // Send an acceptance response if league path is set
        if (leaguePath) {
          const responseData: SkinSyncData = {
            userSkins: Array.from(selectedSkins.values()).map((skin) => ({
              championId: skin.championId,
              skinId: skin.skinId,
              chromaId: skin.chromaId || null,
              fantome: skin.fantome || null,
            })),
            requestType: "accept",
            version: 1,
          };

          // Compress and encode response
          const encodedResponse = compressSkinData(responseData);
          const responseMessage = `[OSS-SKIN-SYNC]${encodedResponse}[/OSS-SKIN-SYNC]`;

          try {
            await invoke("send_lcu_message", {
              leaguePath,
              friendId,
              message: responseMessage,
            });
          } catch (error) {
            console.error("Failed to send acceptance message:", error);
            toast.error(
              `Accepted the request, but couldn't send confirmation to ${friendName}`
            );
          }
        }

        toast.success(`Accepted sync request from ${friendName}`);
      } catch (error) {
        console.error("Failed to accept sync request:", error);
        setPendingSyncRequest(null);
        toast.error(`Failed to accept sync request from ${friendName}`);
      }
    },
    [
      pendingSyncRequest,
      decompressSkinData,
      addPartyMember,
      selectedSkins,
      setPendingSyncRequest,
      compressSkinData,
      leaguePath,
    ]
  );

  // Reject a sync request from a friend
  const rejectSync = useCallback(
    (friendId: string) => {
      // Remove from party if they're in it
      removePartyMember(friendId);

      // If this was rejecting a pending request, clear it and send rejection
      if (pendingSyncRequest && pendingSyncRequest.memberId === friendId) {
        setPendingSyncRequest(null);

        // Send rejection message if league path is set
        if (leaguePath) {
          // Send rejection message
          const rejectionData: SkinSyncData = {
            userSkins: [],
            requestType: "reject",
            version: 1,
          };

          try {
            // Compress and encode rejection
            const encodedRejection = compressSkinData(rejectionData);
            const rejectionMessage = `[OSS-SKIN-SYNC]${encodedRejection}[/OSS-SKIN-SYNC]`;

            // Send the rejection via LCU API
            invoke("send_lcu_message", {
              leaguePath,
              friendId,
              message: rejectionMessage,
            });
          } catch (error) {
            console.error("Failed to send rejection:", error);
          }
        }

        toast.info(`Rejected sync request`);
      } else {
        toast.info(`Removed player from party`);
      }
    },
    [
      pendingSyncRequest,
      removePartyMember,
      setPendingSyncRequest,
      compressSkinData,
      leaguePath,
    ]
  );

  // Handle an incoming message to detect sync requests
  const handleIncomingMessage = useCallback(
    (message: string, senderId: string, senderName: string) => {
      // Check if the message is a skin sync message
      const syncPattern = /\[OSS-SKIN-SYNC\](.*?)\[\/OSS-SKIN-SYNC\]/;
      const match = message.match(syncPattern);

      if (match && match[1]) {
        try {
          const encodedData = match[1];
          const decodedData = decompressSkinData(encodedData);

          switch (decodedData.requestType) {
            case "request":
              // It's a new sync request
              setPendingSyncRequest({
                memberId: senderId,
                memberName: senderName,
                data: encodedData,
              });
              toast.info(`Received sync request from ${senderName}`);
              break;

            case "accept":
              // A friend accepted our request
              if (partyMembers.some((m) => m.id === senderId)) {
                // Create a map of skins from the friend
                const friendSkins = new Map();
                decodedData.userSkins.forEach((skin) => {
                  friendSkins.set(skin.championId, {
                    championId: skin.championId,
                    skinId: skin.skinId,
                    chromaId: skin.chromaId || undefined,
                    fantome: skin.fantome || undefined,
                  });
                });

                // Update the friend's skins
                updatePartyMemberSkins(senderId, friendSkins);
                toast.success(`${senderName} accepted your sync request`);
              }
              break;

            case "reject":
              // A friend rejected our request
              removePartyMember(senderId);
              toast.info(`${senderName} rejected your sync request`);
              break;
          }
        } catch (error) {
          console.error("Failed to process skin sync message:", error);
        }
      }
    },
    [
      decompressSkinData,
      setPendingSyncRequest,
      partyMembers,
      updatePartyMemberSkins,
      removePartyMember,
    ]
  );

  // Function to find a party member's skin for a given champion
  const getPartySkinForChampion = useCallback(
    (championId: number) => {
      // Check if any party member has a skin for this champion
      for (const member of partyMembers) {
        if (member.skins.has(championId)) {
          return member.skins.get(championId);
        }
      }
      return null;
    },
    [partyMembers]
  );

  // Modified game injection logic to include party member skins
  const injectWithPartySkins = useCallback(
    async (championId: number) => {
      try {
        // First, check if the user has selected a skin for this champion
        const userSkin = selectedSkins.get(championId);

        // Then check if any party member has a skin for this champion
        const partySkin = getPartySkinForChampion(championId);

        // If neither the user nor any party member has a skin for this champion, do nothing
        if (!userSkin && !partySkin) {
          return;
        }

        // Prefer user's own skin selection, fall back to party member's skin
        const skinToInject = userSkin || partySkin;

        if (skinToInject && leaguePath) {
          // Use the existing inject_skins Tauri command
          await invoke("inject_skins", {
            app: window.__TAURI__,
            request: {
              league_path: leaguePath,
              skins: [
                {
                  champion_id: skinToInject.championId,
                  skin_id: skinToInject.skinId,
                  chroma_id: skinToInject.chromaId,
                  fantome_path: skinToInject.fantome,
                },
              ],
            },
          });

          if (partySkin && !userSkin) {
            // Notify if using a party member's skin
            toast.info(`Using party member's skin for champion ${championId}`);
          }
        }
      } catch (error) {
        console.error(
          `Failed to inject party skin for champion ${championId}:`,
          error
        );
        toast.error(`Failed to inject skin for champion ${championId}`);
      }
    },
    [selectedSkins, partyMembers, leaguePath, getPartySkinForChampion]
  );

  return {
    activePartyMembers: partyMembers,
    pendingSyncRequest,
    sendSyncRequest,
    acceptSync,
    rejectSync,
    handleIncomingMessage,
    injectWithPartySkins,
    getPartySkinForChampion,
    checkPendingSyncRequests, // Expose the check function for manual triggers
  };
}
