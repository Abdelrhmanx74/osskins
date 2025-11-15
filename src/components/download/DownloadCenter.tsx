"use client";

import { useMemo } from "react";
import { useDownloadsStore, formatBytes, formatSpeed } from "@/lib/store/downloads";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";

interface DownloadCenterProps {
    isOpen: boolean;
    onClose: () => void;
}

export default function DownloadCenter({ isOpen, onClose }: DownloadCenterProps) {
    const items = useDownloadsStore((s) => s.items);
    const order = useDownloadsStore((s) => s.order);
    const clearCompleted = useDownloadsStore((s) => s.clearCompleted);
    const remove = useDownloadsStore((s) => s.remove);

    const list = useMemo(() => order.map((id) => items[id]).filter(Boolean), [order, items]);

    const calcPercent = (d?: number, t?: number) => {
        if (!t || t <= 0 || !d) return undefined;
        return Math.min(100, Math.max(0, (d / t) * 100));
    };

    return (
        <Dialog open={isOpen} onOpenChange={(v) => !v && onClose()}>
            <DialogContent className="max-w-2xl">
                <DialogHeader>
                    <DialogTitle>Downloads</DialogTitle>
                </DialogHeader>

                <div className="flex flex-col gap-3 max-h-[60vh] overflow-y-auto py-2">
                    {list.length === 0 ? (
                        <div className="text-sm text-muted-foreground">No downloads yet.</div>
                    ) : (
                        list.map((item) => {
                            const percent = calcPercent(item.downloaded, item.total);
                            const subtitleParts: string[] = [];
                            if (item.championName) subtitleParts.push(item.championName);
                            if (item.fileName) subtitleParts.push(item.fileName);
                            return (
                                <div key={item.id} className="border rounded-md px-3 py-2 bg-card">
                                    <div className="flex items-center justify-between gap-2">
                                        <div className="text-sm font-medium">
                                            {item.category.toUpperCase()} • {item.status}
                                        </div>
                                        <div className="flex items-center gap-2">
                                            {(item.status === "queued" || item.status === "downloading") && (
                                                <Button
                                                    size="sm"
                                                    variant="ghost"
                                                    onClick={async () => {
                                                        try {
                                                            await invoke("cancel_download", { id: item.id });
                                                        } catch {
                                                            /* ignore */
                                                        }
                                                    }}
                                                >
                                                    Cancel
                                                </Button>
                                            )}
                                            {(item.status === "completed" || item.status === "failed" || item.status === "canceled") && (
                                                <Button size="sm" variant="ghost" onClick={() => remove(item.id)}>
                                                    Dismiss
                                                </Button>
                                            )}
                                        </div>
                                    </div>
                                    {subtitleParts.length > 0 && (
                                        <div className="text-xs text-muted-foreground">{subtitleParts.join(" • ")}</div>
                                    )}
                                    {typeof percent === "number" && (
                                        <div className="mt-2">
                                            <Progress value={percent} />
                                            <div className="flex justify-between text-xs text-muted-foreground mt-1">
                                                <div>
                                                    {formatBytes(item.downloaded)} / {formatBytes(item.total)}
                                                </div>
                                                <div>{formatSpeed(item.speed)}</div>
                                            </div>
                                        </div>
                                    )}
                                    {item.status === "failed" && item.error && (
                                        <div className="text-xs text-red-500 mt-1">{item.error}</div>
                                    )}
                                </div>
                            );
                        })
                    )}
                </div>

                <div className="flex justify-between">
                    <Button variant="secondary" onClick={() => clearCompleted()}>
                        Clear completed
                    </Button>
                    <Button onClick={onClose}>Close</Button>
                </div>
            </DialogContent>
        </Dialog>
    );
}
