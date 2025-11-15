import { create } from "zustand";

export type DownloadCategory = "skin" | "data" | "tools" | "misc";
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

interface DownloadsStore {
    items: Record<string, DownloadItem>;
    order: string[]; // newest first
    upsert: (item: Partial<DownloadItem> & Pick<DownloadItem, "id">) => void;
    clearCompleted: () => void;
    remove: (id: string) => void;
}

export const useDownloadsStore = create<DownloadsStore>((set, get) => ({
    items: {},
    order: [],
    upsert: (partial) => {
        set((state) => {
            const prev = state.items[partial.id];
            const next: DownloadItem = {
                id: partial.id,
                url: partial.url ?? prev?.url ?? "",
                category: (partial.category as DownloadCategory) ?? prev?.category ?? "misc",
                status: (partial.status as DownloadStatus) ?? prev?.status ?? "queued",
                downloaded: partial.downloaded ?? prev?.downloaded,
                total: partial.total ?? prev?.total,
                speed: partial.speed ?? prev?.speed,
                championName: partial.championName ?? prev?.championName ?? null,
                fileName: partial.fileName ?? prev?.fileName ?? null,
                destPath: partial.destPath ?? prev?.destPath ?? null,
                error: partial.error ?? (partial.status === "failed" ? (partial as any).error ?? null : prev?.error) ?? null,
                updatedAt: Date.now(),
            };

            const items = { ...state.items, [next.id]: next };
            const exists = state.order.includes(next.id);
            const order = exists ? state.order : [next.id, ...state.order];
            return { items, order };
        });
    },
    clearCompleted: () => {
        set((state) => {
            const items: Record<string, DownloadItem> = {};
            const order: string[] = [];
            for (const id of state.order) {
                const item = state.items[id];
                if (!item) continue;
                if (item.status === "completed" || item.status === "canceled") continue;
                items[id] = item;
                order.push(id);
            }
            return { items, order };
        });
    },
    remove: (id) => {
        set((state) => {
            const items = { ...state.items };
            delete items[id];
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
