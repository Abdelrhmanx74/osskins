"use client";

import React, { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { partyModeApi } from "@/lib/api/party-mode";
import { toast } from "sonner";
import { usePartyModeStore } from "@/lib/store/party-mode";

interface SkinSentEvent {
  skin_name: string;
  to_friend: string;
}

interface PartyModeProviderProps {
  children: React.ReactNode;
}

export function PartyModeProvider({ children }: PartyModeProviderProps) {
  const loadPairedFriends = usePartyModeStore((s) => s.loadPairedFriends);

  useEffect(() => {
    let unsubscribeFunctions: (() => void)[] = [];

    const initializePartyMode = async () => {
      try {
        console.log("[PartyModeProvider] Initializing party mode...");
        await partyModeApi.startChatMonitor();
        console.log("[PartyModeProvider] Chat monitor started");
        await loadPairedFriends();

        // Set up global event listeners
        const unsubscribeConnection = await partyModeApi.onConnectionRequest(
          (request) => {
            // Instead of showing a toast with a button, directly open the acceptance modal
            document.dispatchEvent(
              new CustomEvent("open-party-mode-acceptance", { detail: request })
            );
          }
        );
        const unsubscribePairingAccepted = await partyModeApi.onPairingAccepted(
          (response) => {
            toast.success(
              `${response.from_summoner_name} accepted your connection request!`
            );
            void loadPairedFriends();
          }
        );
        const unsubscribePairingDeclined = await partyModeApi.onPairingDeclined(
          (response) => {
            toast.error(
              `${response.from_summoner_name} declined your connection request`
            );
          }
        );
        const unsubscribeSkinReceived = await partyModeApi.onSkinReceived(
          (skinShare) => {
            toast.info(
              `🎨 ${skinShare.from_summoner_name} shared ${skinShare.skin_name} for champion ${skinShare.champion_id}`,
              {
                duration: 5000,
              }
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
        unsubscribeFunctions = [
          unsubscribeConnection,
          unsubscribePairingAccepted,
          unsubscribePairingDeclined,
          unsubscribeSkinReceived,
          unsubscribeSkinSent,
        ];
        console.log("[PartyModeProvider] Party mode initialized successfully");
      } catch (error) {
        console.error(
          "[PartyModeProvider] Failed to initialize party mode:",
          error
        );
      }
    };
    void initializePartyMode();
    return () => {
      console.log("[PartyModeProvider] Cleaning up party mode...");
      unsubscribeFunctions.forEach((unsub) => {
        try {
          unsub();
        } catch (error) {
          console.error("Error during party mode cleanup:", error);
        }
      });
    };
  }, []);
  return <>{children}</>;
}
// Store unsubscribe functions
