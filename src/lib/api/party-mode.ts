import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  FriendInfo,
  PairedFriend,
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

  // Add a friend to party mode (simplified approach)
  async addPartyFriend(friendSummonerId: string): Promise<string> {
    console.log(
      "[DEBUG] partyModeApi.addPartyFriend called with:",
      friendSummonerId
    );
    try {
      const result = await invoke("add_party_friend", {
        friendSummonerId,
      });
      console.log("[DEBUG] invoke result:", result);
      return result as string;
    } catch (error) {
      console.error("Failed to add party friend:", error);
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

  // Update party mode settings (only notifications now)
  async updateSettings(notifications: boolean): Promise<void> {
    try {
      await invoke("update_party_mode_settings", {
        notifications,
      });
    } catch (error) {
      console.error("Failed to update party mode settings:", error);
      throw error;
    }
  },

  // Get party mode settings (only notifications now)
  async getSettings(): Promise<{ notifications: boolean }> {
    try {
      const result = await invoke("get_party_mode_settings");
      const notifications = result as boolean;
      return { notifications };
    } catch (error) {
      console.error("Failed to get party mode settings:", error);
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
  onSkinReceived(callback: (skinShare: SkinShare) => void) {
    return listen("party-mode-skin-received", (event) => {
      callback(event.payload as SkinShare);
    });
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
