import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  FriendInfo,
  PairedFriend,
  ConnectionRequest,
  PairingResponse,
  SkinShare,
} from "@/lib/types/party-mode";

// API functions for party mode
export const partyModeApi = {
  // Get friends list from LCU
  async getFriends(): Promise<FriendInfo[]> {
    try {
      const result = await invoke("get_lcu_friends");
      return result as FriendInfo[];
    } catch (error) {
      console.error("Failed to get friends:", error);
      throw error;
    }
  },

  // Send pairing request to a friend
  async sendPairingRequest(friendSummonerId: string): Promise<string> {
    try {
      return await invoke("send_pairing_request", {
        friendSummonerId,
      });
    } catch (error) {
      console.error("Failed to send pairing request:", error);
      throw error;
    }
  },

  // Respond to a pairing request
  async respondToPairingRequest(
    requestId: string,
    friendSummonerId: string,
    accepted: boolean
  ): Promise<void> {
    try {
      await invoke("respond_to_pairing_request", {
        requestId,
        friendSummonerId,
        accepted,
      });
    } catch (error) {
      console.error("Failed to respond to pairing request:", error);
      throw error;
    }
  },

  // Remove a paired friend
  async removePairedFriend(friendSummonerId: string): Promise<void> {
    try {
      await invoke("remove_paired_friend", { friendSummonerId });
    } catch (error) {
      console.error("Failed to remove paired friend:", error);
      throw error;
    }
  },

  // Get list of paired friends
  async getPairedFriends(): Promise<PairedFriend[]> {
    try {
      return await invoke("get_paired_friends");
    } catch (error) {
      console.error("Failed to get paired friends:", error);
      throw error;
    }
  },

  // Update party mode settings
  async updateSettings(
    autoShare: boolean,
    notifications: boolean
  ): Promise<void> {
    try {
      await invoke("update_party_mode_settings", {
        autoShare,
        notifications,
      });
    } catch (error) {
      console.error("Failed to update party mode settings:", error);
      throw error;
    }
  },

  // Start monitoring chat messages for party mode
  async startChatMonitor(): Promise<void> {
    try {
      await invoke("start_party_mode_chat_monitor");
    } catch (error) {
      console.error("Failed to start chat monitor:", error);
      throw error;
    }
  },

  // Event listeners
  onConnectionRequest(callback: (request: ConnectionRequest) => void) {
    return listen("party-mode-connection-request", (event) => {
      callback(event.payload as ConnectionRequest);
    });
  },

  onPairingAccepted(callback: (response: PairingResponse) => void) {
    return listen("party-mode-pairing-accepted", (event) => {
      callback(event.payload as PairingResponse);
    });
  },

  onPairingDeclined(callback: (response: PairingResponse) => void) {
    return listen("party-mode-pairing-declined", (event) => {
      callback(event.payload as PairingResponse);
    });
  },

  onSkinReceived(callback: (skinShare: SkinShare) => void) {
    return listen("party-mode-skin-received", (event) => {
      callback(event.payload as SkinShare);
    });
  },

  // Test functions for simulating party mode scenarios
  async simulatePartyModeTest(): Promise<void> {
    try {
      await invoke("simulate_party_mode_test");
    } catch (error) {
      console.error("Failed to simulate party mode test:", error);
      throw error;
    }
  },

  async simulateMultipleSkinShares(): Promise<void> {
    try {
      await invoke("simulate_multiple_skin_shares");
    } catch (error) {
      console.error("Failed to simulate multiple skin shares:", error);
      throw error;
    }
  },

  async clearTestData(): Promise<void> {
    try {
      await invoke("clear_test_data");
    } catch (error) {
      console.error("Failed to clear test data:", error);
      throw error;
    }
  },

  async simulateMultipleSkinInjection(): Promise<void> {
    try {
      await invoke("simulate_multiple_skin_injection");
    } catch (error) {
      console.error("Failed to simulate multiple skin injection:", error);
      throw error;
    }
  },
};

// Helper function to get status color for friend availability
export const getStatusColor = (availability?: string, isOnline?: boolean) => {
  if (!isOnline) return "bg-gray-500";
  if (availability === "online") return "bg-green-500";
  if (availability === "away") return "bg-yellow-500";
  return "bg-gray-500";
};

// Helper function to get status text for friend availability
export const getStatusText = (availability?: string, isOnline?: boolean) => {
  if (!isOnline) return "Offline";
  return availability ?? "Unknown";
};
