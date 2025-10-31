"use client";

import React, { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { partyModeApi } from "@/lib/api/party-mode";
import { toast } from "sonner";
import { usePartyModeStore } from "@/lib/store/party-mode";
import { useGameStore } from "@/lib/store";

interface SkinSentEvent {
  skin_name: string;
  to_friend: string;
}

interface PartyModeProviderProps {
  children: React.ReactNode;
}

export function PartyModeProvider({ children }: PartyModeProviderProps) {
  const loadPairedFriends = usePartyModeStore((s) => s.loadPairedFriends);
  const manualInjectionMode = useGameStore((s) => s.manualInjectionMode);

  useEffect(() => {
    // Keep track of unsubscribers so we can clean up when manual mode toggles
    let unsubscribeFunctions: (() => void)[] = [];

    const cleanup = () => {
      if (unsubscribeFunctions.length > 0) {
        console.log("[PartyModeProvider] Cleaning up party mode listeners...");
        unsubscribeFunctions.forEach((unsub) => {
          try {
            unsub();
          } catch (error) {
            console.error("Error during party mode cleanup:", error);
          }
        });
        unsubscribeFunctions = [];
      }
    };

    const initializePartyMode = async () => {
      // If manual injection mode is enabled, don't initialize party mode
      if (manualInjectionMode) {
        console.log("[PartyModeProvider] Manual injection mode is active — skipping party mode initialization.");
        cleanup();
        return;
      }

      try {
        console.log("[PartyModeProvider] Initializing party mode...");
        await partyModeApi.startChatMonitor();
        console.log("[PartyModeProvider] Chat monitor started");
        await loadPairedFriends();

        // Set up global event listeners
        const unsubscribeSkinReceived = await partyModeApi.onSkinReceived(
          (skinShare) => {
            toast.info(
              `🎨 ${skinShare.from_summoner_name} shared ${skinShare.skin_name}`,
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

        const unsubscribePairedFriendsUpdated = await listen(
          "party-mode-paired-friends-updated",
          () => {
            // Reload paired friends when they're updated
            void loadPairedFriends().catch(() => { });
            return undefined;
          }
        );

        unsubscribeFunctions = [
          unsubscribeSkinReceived,
          unsubscribeSkinSent,
          unsubscribePairedFriendsUpdated,
        ];
        console.log("[PartyModeProvider] Party mode initialized successfully");
      } catch (error) {
        console.error(
          "[PartyModeProvider] Failed to initialize party mode:",
          error
        );
        cleanup();
      }
    };

    // Initialize or cleanup whenever manualInjectionMode changes
    void initializePartyMode();

    return () => { cleanup(); };
    // We intentionally re-run when manualInjectionMode changes so we can teardown/init accordingly
  }, [manualInjectionMode, loadPairedFriends]);
  return <>{children}</>;
}
// Store unsubscribe functions
