import { create } from "zustand";
import { partyModeApi } from "@/lib/api/party-mode";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type {
  PairedFriend,
  SkinShare,
} from "@/lib/types/party-mode";

interface SkinSentEvent {
  skin_name: string;
  to_friend: string;
}

interface PartyModeState {
  pairedFriends: PairedFriend[];
  isInitialized: boolean;
  init: () => Promise<void>;
  loadPairedFriends: () => Promise<void>;
}

export const usePartyModeStore = create<PartyModeState>((set, get) => ({
  pairedFriends: [],
  isInitialized: false,
  loadPairedFriends: async () => {
    try {
      const friends = await partyModeApi.getPairedFriends();
      set({ pairedFriends: friends });
    } catch (error) {
      console.error("Failed to load paired friends:", error);
    }
  },
  init: async () => {
    if (get().isInitialized) return;
    try {
      await partyModeApi.startChatMonitor();
      await get().loadPairedFriends();
      
      // Set up global event listeners
      const unsubscribeSkinReceived = await partyModeApi.onSkinReceived(
        (skinShare: SkinShare) => {
          toast.info(
            `🎨 ${skinShare.from_summoner_name} shared ${skinShare.skin_name} for champion ${skinShare.champion_id}`,
            { duration: 5000 }
          );
        }
      );
      
      const unsubscribeSkinSent = await listen<SkinSentEvent>(
        "party-mode-skin-sent",
        (event) => {
          const data = event.payload;
          toast.success(`📤 Sent ${data.skin_name} to ${data.to_friend}`, {
            duration: 4000,
          });
        }
      );
      
      const unsubscribePairedFriendsUpdated = await listen(
        "party-mode-paired-friends-updated",
        () => {
          void get().loadPairedFriends();
        }
      );
      
      // Store unsubscribers if you want to clean up later (not shown here)
      set({ isInitialized: true });
    } catch (error) {
      console.error("[PartyMode] Failed to initialize party mode:", error);
    }
  },
}));
