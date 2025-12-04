import { create } from "zustand";

export type DownloadCategory = "skin" | "data" | "tools" | "misc" | "batch";
export type DownloadStatus = "queued" | "downloading" | "completed" | "failed" | "canceled";

export interface DownloadItem {
    id: string;
    url: string;
    category: DownloadCategory;
    status: DownloadStatus;
    downloaded?: number;
    total?: number;
    speed?: number;
    championName?: string | null;
    fileName?: string | null;
    destPath?: string | null;
    error?: string | null;
    updatedAt: number;
}

// New batch download progress type
export interface BatchDownloadProgress {
    batchId: string;
    totalItems: number;
    completedItems: number;
    failedItems: number;
    currentItems: string[];
    totalBytes: number;
    downloadedBytes: number;
    speed: number;
    status: "downloading" | "completed" | "failed" | "canceled";
}

// Skin download request type (mirrors Rust struct)
export interface SkinDownloadRequest {
    championId: number;
    skinId: number;
    chromaId?: number;
    formId?: number;
    championName: string;
    fileName: string;
}

interface DownloadsStore {
    items: Record<string, DownloadItem>;
    order: string[]; // newest first
    // Batch download state
    activeBatch: BatchDownloadProgress | null;
    upsert: (item: Partial<DownloadItem> & Pick<DownloadItem, "id">) => void;
    updateBatch: (progress: BatchDownloadProgress) => void;
    clearBatch: () => void;
    clearCompleted: () => void;
    remove: (id: string) => void;
}

export const useDownloadsStore = create<DownloadsStore>((set, get) => ({
    items: {},
    order: [],
    activeBatch: null,
    upsert: (partial) => {
        set((state) => {
            const prev = state.items[partial.id];
            const next: DownloadItem = {
                id: partial.id,
                url: partial.url ?? prev?.url ?? "",
                category: (partial.category as DownloadCategory) ?? prev?.category ?? "misc",
                status: (partial.status as DownloadStatus) ?? prev?.status ?? "queued",
                downloaded: partial.downloaded ?? (prev?.downloaded ?? undefined),
                total: partial.total ?? (prev?.total ?? undefined),
                speed: partial.speed ?? (prev?.speed ?? undefined),
                championName: partial.championName ?? (prev?.championName ?? null),
                fileName: partial.fileName ?? (prev?.fileName ?? null),
                destPath: partial.destPath ?? (prev?.destPath ?? null),
                error: partial.error ?? (partial.status === "failed" ? (partial as any).error ?? null : (prev?.error ?? null)),
                updatedAt: Date.now(),
            };

            const items = { ...state.items, [next.id]: next };
            const exists = state.order.includes(next.id);
            const order = exists ? state.order : [next.id, ...state.order];
            return { items, order };
        });
    },
    updateBatch: (progress) => {
        set({ activeBatch: progress });
    },
    clearBatch: () => {
        set({ activeBatch: null });
    },
    clearCompleted: () => {
        set((state) => {
            const items: Record<string, DownloadItem> = {};
            const order: string[] = [];
            for (const id of state.order) {
                const item = state.items[id];
                if (!item || item.status === "completed" || item.status === "canceled") continue;
                items[id] = item;
                order.push(id);
            }
            return { items, order };
        });
    },
    remove: (id) => {
        set((state) => {
            const { [id]: _, ...items } = state.items;
            const order = state.order.filter((x) => x !== id);
            return { items, order };
        });
    },
}));

export function formatBytes(bytes?: number): string {
    if (!bytes || bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"] as const;
    const e = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const v = bytes / 1024 ** e;
    return `${v.toFixed(v >= 10 ? 0 : 1)} ${units[e]}`;
}

export function formatSpeed(speed?: number): string {
    if (!speed || speed <= 0) return "";
    return `${formatBytes(speed)}/s`;
}
