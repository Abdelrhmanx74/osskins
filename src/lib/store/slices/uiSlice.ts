import { StateCreator } from "zustand";

export type SkinTab = "official" | "custom";

export interface UISlice {
    activeTab: SkinTab;
    showUpdateModal: boolean;
    manualInjectionMode: boolean;
    setActiveTab: (tab: SkinTab) => void;
    setShowUpdateModal: (v: boolean) => void;
    setManualInjectionMode: (v: boolean) => void;
}

export const createUISlice: StateCreator<UISlice> = (set) => ({
    activeTab: "official",
    showUpdateModal: false,
    manualInjectionMode: false,
    setActiveTab: (tab) => {
        set({ activeTab: tab });
        if (typeof window !== "undefined") {
            localStorage.setItem("activeSkinsTab", tab);
        }
    },
    setShowUpdateModal: (v: boolean) => {
        set(() => ({ showUpdateModal: v }));
    },
    setManualInjectionMode: (v: boolean) => {
        localStorage.setItem("manualInjectionMode", v ? "true" : "false");
        set({ manualInjectionMode: v });
    },
});
