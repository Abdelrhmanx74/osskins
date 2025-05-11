import { create } from "zustand";
import { CustomSkin } from "./types";

interface SelectedSkin {
  championId: number;
  skinId: number;
  chromaId?: number;
  fantome?: string; // Add fantome path
}

// Define the possible injection statuses
export type InjectionStatus = "idle" | "injecting" | "success" | "error";

// Custom skin tabs
export type SkinTab = "official" | "custom";

// Party member interface
export interface PartyMember {
  id: string;
  name: string;
  availability: "online" | "away" | "offline" | "in-game";
  skins: Map<number, SelectedSkin>; // Map of champion ID to selected skin
}

interface GameState {
  leaguePath: string | null;
  lcuStatus: string | null;
  injectionStatus: InjectionStatus;
  selectedSkins: Map<number, SelectedSkin>;
  favorites: Set<number>;
  hasCompletedOnboarding: boolean;
  activeTab: SkinTab;
  customSkins: Map<number, CustomSkin[]>;
  // Data update settings
  autoUpdateData: boolean;
  hasNewDataUpdate: boolean;
  // Party mode state
  partyMembers: PartyMember[];
  pendingSyncRequest: {
    memberId: string;
    memberName: string;
    data: string;
  } | null;
  // Methods
  setLeaguePath: (path: string) => void;
  setLcuStatus: (status: string) => void;
  setInjectionStatus: (status: InjectionStatus) => void;
  selectSkin: (
    championId: number,
    skinId: number,
    chromaId?: number,
    fantome?: string
  ) => void;
  clearSelection: (championId: number) => void;
  clearAllSelections: () => void;
  toggleFavorite: (championId: number) => void;
  setFavorites: (favorites: Set<number>) => void;
  setHasCompletedOnboarding: (completed: boolean) => void;
  setActiveTab: (tab: SkinTab) => void;
  addCustomSkin: (skin: CustomSkin) => void;
  removeCustomSkin: (skinId: string) => void;
  setCustomSkins: (skins: CustomSkin[]) => void;
  // Data update methods
  setAutoUpdateData: (autoUpdate: boolean) => void;
  setHasNewDataUpdate: (hasUpdate: boolean) => void;
  // Party mode methods
  addPartyMember: (member: PartyMember) => void;
  removePartyMember: (memberId: string) => void;
  updatePartyMemberSkins: (
    memberId: string,
    skins: Map<number, SelectedSkin>
  ) => void;
  clearParty: () => void;
  setPendingSyncRequest: (
    request: { memberId: string; memberName: string; data: string } | null
  ) => void;
}

export const useGameStore = create<GameState>((set) => ({
  leaguePath: null,
  lcuStatus: null,
  injectionStatus: "idle", // Default status
  selectedSkins: new Map(),
  favorites: new Set(),
  hasCompletedOnboarding: false,
  activeTab: "official", // Default to official skins tab
  customSkins: new Map(),
  // Data update settings
  autoUpdateData: true, // Default to auto-update enabled
  hasNewDataUpdate: false,
  // Party mode state
  partyMembers: [],
  pendingSyncRequest: null,
  setLeaguePath: (path) => {
    set({ leaguePath: path });
  },
  setLcuStatus: (status) => {
    set({ lcuStatus: status });
  },
  setInjectionStatus: (status) => {
    // Add implementation
    set({ injectionStatus: status });
  },
  selectSkin: (championId, skinId, chromaId, fantome) => {
    set((state) => {
      const newSelectedSkins = new Map(state.selectedSkins);
      newSelectedSkins.set(championId, {
        championId,
        skinId,
        chromaId,
        fantome,
      });
      return { selectedSkins: newSelectedSkins };
    });
  },
  clearSelection: (championId) => {
    set((state) => {
      const newSelectedSkins = new Map(state.selectedSkins);
      newSelectedSkins.delete(championId);
      return { selectedSkins: newSelectedSkins };
    });
  },
  clearAllSelections: () => {
    set({ selectedSkins: new Map() });
  },
  toggleFavorite: (championId) => {
    set((state) => {
      const newFavorites = new Set(state.favorites);
      if (newFavorites.has(championId)) {
        newFavorites.delete(championId);
      } else {
        newFavorites.add(championId);
      }
      return { favorites: newFavorites };
    });
  },
  setFavorites: (favorites) => {
    set({ favorites });
  },
  setHasCompletedOnboarding: (completed) => {
    set({ hasCompletedOnboarding: completed });
    if (typeof window !== "undefined") {
      localStorage.setItem("hasCompletedOnboarding", completed.toString());
    }
  },
  setActiveTab: (tab) => {
    set({ activeTab: tab });
    if (typeof window !== "undefined") {
      localStorage.setItem("activeSkinsTab", tab);
    }
  },
  addCustomSkin: (skin) => {
    set((state) => {
      const newCustomSkins = new Map(state.customSkins);
      const championId = skin.champion_id;
      const existingSkins = newCustomSkins.get(championId) ?? [];
      newCustomSkins.set(championId, [...existingSkins, skin]);
      return { customSkins: newCustomSkins };
    });
  },
  removeCustomSkin: (skinId) => {
    set((state) => {
      const newCustomSkins = new Map(state.customSkins);

      // Find which champion has this skin
      for (const [championId, skins] of newCustomSkins.entries()) {
        const updatedSkins = skins.filter((skin) => skin.id !== skinId);

        if (updatedSkins.length !== skins.length) {
          // We found and removed the skin
          if (updatedSkins.length === 0) {
            newCustomSkins.delete(championId);
          } else {
            newCustomSkins.set(championId, updatedSkins);
          }
          break;
        }
      }

      return { customSkins: newCustomSkins };
    });
  },
  setCustomSkins: (skins) => {
    set(() => {
      const customSkinsMap = new Map<number, CustomSkin[]>();

      // Group skins by champion ID
      skins.forEach((skin) => {
        const championId = skin.champion_id;
        const existingSkins = customSkinsMap.get(championId) ?? [];
        customSkinsMap.set(championId, [...existingSkins, skin]);
      });

      return { customSkins: customSkinsMap };
    });
  },
  // Data update methods
  setAutoUpdateData: (autoUpdate) => {
    set({ autoUpdateData: autoUpdate });
    if (typeof window !== "undefined") {
      localStorage.setItem("autoUpdateData", autoUpdate.toString());
    }
  },
  setHasNewDataUpdate: (hasUpdate) => {
    set({ hasNewDataUpdate: hasUpdate });
  },
  // Party mode methods
  addPartyMember: (member) => {
    set((state) => {
      // Don't add duplicates
      if (state.partyMembers.some((m) => m.id === member.id)) {
        return state;
      }
      // Max party size is 5 (including the user)
      if (state.partyMembers.length >= 4) {
        return state;
      }
      return { partyMembers: [...state.partyMembers, member] };
    });
  },
  removePartyMember: (memberId) => {
    set((state) => ({
      partyMembers: state.partyMembers.filter((m) => m.id !== memberId),
    }));
  },
  updatePartyMemberSkins: (memberId, skins) => {
    set((state) => ({
      partyMembers: state.partyMembers.map((member) =>
        member.id === memberId ? { ...member, skins } : member
      ),
    }));
  },
  clearParty: () => {
    set({ partyMembers: [] });
  },
  setPendingSyncRequest: (request) => {
    set({ pendingSyncRequest: request });
  },
}));

// Terminal log store
export interface TerminalLog {
  message: string;
  log_type: string;
  timestamp: string;
}

interface TerminalLogState {
  logs: TerminalLog[];
  addLog: (log: TerminalLog) => void;
  clearLogs: () => void;
}

export const useTerminalLogStore = create<TerminalLogState>((set) => ({
  logs: [],
  addLog: (log) => {
    set((state) => ({ logs: [...state.logs, log] }));
  },
  clearLogs: () => {
    set({ logs: [] });
  },
}));
