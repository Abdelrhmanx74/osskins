import { StateCreator } from "zustand";

export type InjectionStatus =
  | "idle"
  | "injecting"
  | "busy"
  | "running"
  | "patching"
  | "success"
  | "error";

export interface GameSlice {
  leaguePath: string | null;
  lcuStatus: string | null;
  injectionStatus: InjectionStatus;
  lastInjectionError: string | null;
  hasCompletedOnboarding: boolean;
  setLeaguePath: (path: string) => void;
  setLcuStatus: (status: string) => void;
  setInjectionStatus: (status: InjectionStatus) => void;
  setLastInjectionError: (message: string | null) => void;
  setHasCompletedOnboarding: (completed: boolean) => void;
}

export const createGameSlice: StateCreator<GameSlice> = (set) => ({
  leaguePath: null,
  lcuStatus: null,
  injectionStatus: "idle",
  lastInjectionError: null,
  hasCompletedOnboarding: false,
  setLeaguePath: (path) => {
    set({ leaguePath: path });
  },
  setLcuStatus: (status) => {
    set({ lcuStatus: status });
  },
  setInjectionStatus: (status) => {
    set({ injectionStatus: status });
  },
  setLastInjectionError: (message) => {
    set({ lastInjectionError: message });
  },
  setHasCompletedOnboarding: (completed) => {
    set({ hasCompletedOnboarding: completed });
    if (typeof window !== "undefined") {
      localStorage.setItem("hasCompletedOnboarding", completed.toString());
    }
  },
});
