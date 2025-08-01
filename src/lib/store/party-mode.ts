import { create } from "zustand";
import { partyModeApi } from "@/lib/api/party-mode";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type {
  PairedFriend,
  ConnectionRequest,
  PairingResponse,
  SkinShare,
} from "@/lib/types/party-mode";

interface SkinSentEvent {
  skin_name: string;
  to_friend: string;
}

interface PartyModeState {
  pairedFriends: PairedFriend[];
  isInitialized: boolean;
  incomingRequest: ConnectionRequest | null;
  setIncomingRequest: (req: ConnectionRequest | null) => void;
  showConnectionRequest: boolean;
  setShowConnectionRequest: (show: boolean) => void;
  init: () => Promise<void>;
  loadPairedFriends: () => Promise<void>;
}

export const usePartyModeStore = create<PartyModeState>((set, get) => ({
  pairedFriends: [],
  isInitialized: false,
  incomingRequest: null,
  showConnectionRequest: false,
  setIncomingRequest: (req) => {
    set({ incomingRequest: req });
  },
  setShowConnectionRequest: (show) => {
    set({ showConnectionRequest: show });
  },
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
      const unsubscribeConnection = await partyModeApi.onConnectionRequest(
        (request: ConnectionRequest) => {
          set({ incomingRequest: request, showConnectionRequest: true });
        }
      );
      const unsubscribePairingAccepted = await partyModeApi.onPairingAccepted(
        (response: PairingResponse) => {
          toast.success(
            `${response.from_summoner_name} accepted your connection request!`
          );
          void get().loadPairedFriends();
        }
      );
      const unsubscribePairingDeclined = await partyModeApi.onPairingDeclined(
        (response: PairingResponse) => {
          toast.error(
            `${response.from_summoner_name} declined your connection request`
          );
        }
      );
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
      // Store unsubscribers if you want to clean up later (not shown here)
      set({ isInitialized: true });
    } catch (error) {
      console.error("[PartyMode] Failed to initialize party mode:", error);
    }
  },
}));
