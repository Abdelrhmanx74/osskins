import { create } from "zustand";

export type ToolsPhase =
  | "idle"
  | "checking"
  | "downloading"
  | "installing"
  | "completed"
  | "skipped"
  | "error";

export type ToolsSource = "auto" | "manual";

export interface ToolsProgressState {
  source: ToolsSource;
  phase: ToolsPhase;
  progress: number;
  message?: string;
  downloaded?: number;
  total?: number;
  speed?: number;
  error?: string;
  updatedAt: number;
}

export interface ToolsStatusState {
  installed: boolean;
  version: string | null;
  latestVersion: string | null;
  hasUpdate: boolean;
  path: string | null;
  downloadSize: number | null;
  lastChecked: number | null;
}

interface ToolsStore {
  progress: Record<ToolsSource, ToolsProgressState | null>;
  status: ToolsStatusState | null;
  setProgress: (
    source: ToolsSource,
    progress: ToolsProgressState | null
  ) => void;
  mergeProgress: (
    source: ToolsSource,
    partial: Partial<Omit<ToolsProgressState, "source" | "updatedAt">> & {
      phase?: ToolsPhase;
    }
  ) => void;
  clearProgress: (source?: ToolsSource) => void;
  updateStatus: (partial: Partial<ToolsStatusState>) => void;
}

const defaultStatus: ToolsStatusState = {
  installed: false,
  version: null,
  latestVersion: null,
  hasUpdate: false,
  path: null,
  downloadSize: null,
  lastChecked: null,
};

export const useToolsStore = create<ToolsStore>((set, get) => ({
  progress: {
    auto: null,
    manual: null,
  },
  status: null,
  setProgress: (source, progress) => {
    set((state) => ({
      progress: {
        ...state.progress,
        [source]: progress,
      },
    }));
  },
  mergeProgress: (source, partial) => {
    set((state) => {
      const previous =
        state.progress[source] ??
        ({
          source,
          phase: partial.phase ?? "idle",
          progress: partial.progress ?? 0,
          message: partial.message,
          downloaded: partial.downloaded,
          total: partial.total,
          speed: partial.speed,
          error: partial.error,
          updatedAt: Date.now(),
        } satisfies ToolsProgressState);

      const nextPhase = partial.phase ?? previous.phase;
      const nextProgress =
        typeof partial.progress === "number"
          ? partial.progress
          : previous.progress;
      const nextError = (() => {
        if (partial.phase && partial.phase !== "error") {
          return undefined;
        }
        if (partial.error !== undefined) {
          return partial.error;
        }
        return previous.error;
      })();

      const next: ToolsProgressState = {
        source,
        phase: nextPhase,
        progress: nextProgress,
        message: partial.message ?? previous.message,
        downloaded: partial.downloaded ?? previous.downloaded,
        total: partial.total ?? previous.total,
        speed: partial.speed ?? previous.speed,
        error: nextError,
        updatedAt: Date.now(),
      };

      return {
        progress: {
          ...state.progress,
          [source]: next,
        },
      };
    });
  },
  clearProgress: (source) => {
    if (source) {
      set((state) => ({
        progress: {
          ...state.progress,
          [source]: null,
        },
      }));
    } else {
      set({
        progress: {
          auto: null,
          manual: null,
        },
      });
    }
  },
  updateStatus: (partial) => {
    set((state) => ({
      status: {
        ...(state.status ?? defaultStatus),
        ...partial,
      },
    }));
  },
}));
