"use client";

import React, { useCallback, useState, useEffect } from "react";
import UpdateModal from "@/components/download/UpdateModal";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useToolsStore } from "@/lib/store/tools";
import type { EnsureModToolsResult, CslolManagerStatus } from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useI18n } from "@/lib/i18n";
import { Badge } from "./ui/badge";

interface Props {
    isOpen: boolean;
    onClose: () => void;
}

export default function CslolManagerModal({ isOpen, onClose }: Props) {
    const { t } = useI18n();
    const toolsStatus = useToolsStore((s) => s.status);
    const autoProgress = useToolsStore((s) => s.progress.auto);
    const manualProgress = useToolsStore((s) => s.progress.manual);
    const updateToolsStatus = useToolsStore((s) => s.updateStatus);
    const clearProgress = useToolsStore((s) => s.clearProgress);

    const [loading, setLoading] = useState(false);
    const [done, setDone] = useState(false);

    const refresh = useCallback(async () => {
        setLoading(true);
        setDone(false);
        try {
            const status = await invoke<CslolManagerStatus>("get_cslol_manager_status");
            updateToolsStatus({
                installed: status.installed,
                version: status.version ?? null,
                latestVersion: status.latestVersion ?? null,
                hasUpdate: status.hasUpdate,
                path: status.path ?? null,
                downloadSize: status.downloadSize ?? null,
                lastChecked: Date.now(),
            });
            toast.success("Refreshed status");
        } catch (e) {
            const errorMessage = e instanceof Error ? e.message : String(e);
            console.error("Failed to refresh cslol status:", errorMessage);
            toast.error(errorMessage || "Failed to refresh status");
        } finally {
            setLoading(false);
        }
    }, [updateToolsStatus]);

    const install = useCallback(async () => {
        clearProgress("manual");
        setLoading(true);
        setDone(false);
        try {
            const res = await invoke<EnsureModToolsResult>("ensure_mod_tools", { force: true });
            updateToolsStatus({
                installed: res.installed,
                version: res.version ?? null,
                latestVersion: res.latestVersion ?? null,
                hasUpdate: Boolean(res.latestVersion && res.version && res.version !== res.latestVersion),
                path: res.path ?? null,
                lastChecked: Date.now(),
            });
            toast.success(res.updated ? "Tools updated" : "Tools installed");
            setDone(true);
            // Clear progress so it doesn't reappear after done
            clearProgress("manual");
            setTimeout(() => { setDone(false); }, 2000);
        } catch (e) {
            const errorMessage = e instanceof Error ? e.message : String(e);
            console.error("Failed to install tools:", errorMessage);
            toast.error(errorMessage || "Failed to install tools");
        } finally {
            setLoading(false);
        }
    }, [clearProgress, updateToolsStatus]);

    useEffect(() => {
        if (isOpen) {
            setDone(false);
            void refresh();
        }
    }, [isOpen, refresh]);

    const progress = manualProgress ?? autoProgress;
    const progressValue = progress
        ? progress.phase === "completed" || progress.phase === "skipped"
            ? 100
            : progress.progress
        : 0;

    const showProgress = progress && !done && progress.phase !== "completed" && progress.phase !== "skipped";

    const primaryAction = { label: toolsStatus?.installed ? (toolsStatus.hasUpdate ? "Update Tools" : "Reinstall Tools") : "Install Tools", onClick: install, disabled: loading };
    const secondaryAction = { label: "Refresh", onClick: refresh, disabled: loading };

    const pillLabel = toolsStatus?.installed ? (toolsStatus.version ?? "unknown") : t("tools.manager.not_installed");
    // show badge only; avoid duplicating the textual 'Up to date' in the pill subtext
    const pillSub = undefined;
    const pillLoading = Boolean(loading || showProgress);
    const pillBadge = toolsStatus?.installed ? (toolsStatus.hasUpdate ? t("tools.manager.update_available") : t("tools.manager.up_to_date")) : undefined;
    const pillBadgeVariant: "secondary" | "destructive" | "default" | undefined = toolsStatus?.installed ? (toolsStatus.hasUpdate ? "destructive" : "secondary") : undefined;

    return (
        <UpdateModal
            isOpen={isOpen}
            title={t("tools.manager.title")}
            statusMessage={undefined}
            isBusy={loading}
            progress={showProgress ? { value: progressValue, processedChampions: undefined, totalChampions: undefined, currentChampion: undefined } : null}
            items={null}
            commit={null}
            pill={{ label: pillLabel, sub: pillSub, loading: pillLoading, badge: pillBadge, badgeVariant: pillBadgeVariant }}
            primaryAction={primaryAction}
            secondaryAction={secondaryAction}
            onClose={onClose}
        >
            <div className="grid grid-cols-2 text-sm text-muted-foreground items-center justify-between">
                <div>Installed version</div>
                <div className="font-mono flex items-center justify-between">
                    {toolsStatus?.installed ? toolsStatus.version ?? "unknown" : t("tools.manager.not_installed")}
                    {toolsStatus?.installed && !toolsStatus.hasUpdate && (
                        <Badge variant={"secondary"}>{t("tools.manager.up_to_date")}</Badge>
                    )}
                    {toolsStatus?.installed && toolsStatus.hasUpdate && (
                        <Badge variant={"destructive"}>{t("tools.manager.update_available")}</Badge>
                    )}
                </div>
                <div>Latest version</div>
                <div className="font-mono">{toolsStatus?.latestVersion ?? "unknown"}</div>
                {toolsStatus?.downloadSize ? (
                    <>
                        <div>Download size</div>
                        <div>{(toolsStatus.downloadSize / (1024 * 1024)).toFixed(0)} MB</div>
                    </>
                ) : null}
                <div>Location</div>
                <div className="line-clamp-2">{toolsStatus?.path ?? "-"}</div>
            </div>
            {showProgress && (
                <div className="space-y-2 mt-3">
                    <Progress value={progressValue} />
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                        <span>{Math.round(progressValue)}%</span>
                        {progress.downloaded != null && progress.total != null ? (
                            <span>
                                {Math.round(progress.downloaded / 1024 / 1024)} MB / {Math.round(progress.total / 1024 / 1024)} MB
                            </span>
                        ) : null}
                    </div>
                </div>
            )}
        </UpdateModal>
    );
}
