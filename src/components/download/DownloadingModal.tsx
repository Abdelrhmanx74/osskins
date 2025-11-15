"use client";

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import type {
    DataUpdateProgress,
    DataUpdateResult,
} from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { getLolSkinsManifest, getLolSkinsManifestCommit } from "@/lib/data-utils";
import { Loader2 } from "lucide-react";
import { useCallback, useEffect, useState, useTransition, useRef } from "react";
import UpdateModal from "./UpdateModal";
import { toast } from "sonner";

const formatBytes = (bytes: number): string => {
    if (!Number.isFinite(bytes) || bytes <= 0) {
        return "0 B";
    }

    const units = ["B", "KB", "MB", "GB", "TB"] as const;
    const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / 1024 ** exponent;
    return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[exponent]}`;
};

const formatSpeed = (bytesPerSecond?: number): string => {
    if (!bytesPerSecond || bytesPerSecond <= 0) {
        return "";
    }
    return `${formatBytes(bytesPerSecond)}/s`;
};



interface DownloadingModalProps {
    isOpen: boolean;
    onClose: () => void;
    progress: DataUpdateProgress | null;
    onUpdateData: () => Promise<void>;
    onReinstallData: () => Promise<void>;
    isUpdating: boolean;
}

export function DownloadingModal({
    isOpen,
    onClose,
    progress,
    onUpdateData,
    onReinstallData,
    isUpdating,
}: DownloadingModalProps) {
    const [isPending, startTransition] = useTransition();
    const [updateResult, setUpdateResult] = useState<DataUpdateResult | null>(
        null,
    );
    const [checkingForUpdates, setCheckingForUpdates] = useState(false);
    const [updatingData, setUpdatingData] = useState(false);
    const [isReinstalling, setIsReinstalling] = useState(false);
    const [manifestCommit, setManifestCommit] = useState<string | null>(null);
    const [manifestRepo, setManifestRepo] = useState<string | null>(null);
    const [manifestGeneratedAt, setManifestGeneratedAt] = useState<string | null>(null);

    const isBusy = isUpdating || updatingData || isReinstalling;
    const modalBusy = isBusy;

    // Check for GitHub updates when the modal is opened
    useEffect(() => {
        if (isOpen) {
            checkForUpdates();
            // attempt to fetch manifest commit info for display
            void (async () => {
                try {
                    const manifest = await getLolSkinsManifest();
                    if (manifest) {
                        const commit = getLolSkinsManifestCommit(manifest);
                        setManifestCommit(commit);
                        try {
                            const repoParts = manifest.source_repo.split("@");
                            // repoParts[0] typically contains owner/repo
                            setManifestRepo(repoParts[0]);
                        } catch {
                            // ignore
                        }
                        setManifestGeneratedAt(manifest.generated_at || null);
                        // commit sha captured; UI will display short sha if available
                    }
                } catch (err) {
                    console.debug("Failed to load manifest for commit info:", err);
                }
            })();
        }
    }, [isOpen]);

    // Function to check for updates from GitHub
    // Uses a request-id + timeout to avoid leaving the UI stuck when network is down.
    const checkRequestId = useRef(0);
    const checkForUpdates = () => {
        const id = ++checkRequestId.current;
        setCheckingForUpdates(true);

        startTransition(async () => {
            let timerId: number | null = null;
            try {
                const p = invoke<DataUpdateResult>("check_data_updates");
                const timeoutPromise = new Promise<never>((_res, rej) => {
                    // Give the backend a bit more time but still avoid hanging the UI indefinitely
                    timerId = window.setTimeout(() => { rej(new Error("timeout")); }, 15000);
                });
                const result = await Promise.race([p, timeoutPromise]);
                // ignore late responses from previous requests
                if (id !== checkRequestId.current) return;
                setUpdateResult(result);
                console.log("Update check result:", result);
            } catch (error) {
                console.error("Failed to check for updates:", error);
                if ((error as Error).message === "timeout") {
                    toast.error("Data update check timed out — network may be slow or blocked");
                } else {
                    toast.error("Failed to check for updates");
                }
            } finally {
                if (typeof timerId === "number") {
                    clearTimeout(timerId);
                }
                // only clear the visible checking state if this is the latest request
                if (id === checkRequestId.current) setCheckingForUpdates(false);
            }
        });
    };

    // Function to pull updates from GitHub
    const pullUpdates = () => {
        setUpdatingData(true);

        startTransition(async () => {
            try {
                const updateToast = toast.loading("Updating champion data...");
                await onUpdateData();
                toast.dismiss(updateToast);
                toast.success("Data update triggered successfully");
                // Refresh status after update kicks off
                checkForUpdates();
            } catch (error) {
                console.error("Failed to update data:", error);
                toast.error(`Failed to update data: ${String(error)}`);
            } finally {
                setUpdatingData(false);
            }
        });
    };

    const handleReinstall = () => {
        setIsReinstalling(true);

        startTransition(async () => {
            const reinstallToast = toast.loading("Reinstalling champion data...");
            try {
                await onReinstallData();
                toast.dismiss(reinstallToast);
                toast.success("Champion data reinstalled successfully");
                checkForUpdates();
            } catch (error) {
                toast.dismiss(reinstallToast);
                console.error("Failed to reinstall data:", error);
                toast.error(`Failed to reinstall data: ${String(error)}`);
            } finally {
                setIsReinstalling(false);
            }
        });
    };

    const shortCommit = manifestCommit ? manifestCommit.slice(0, 8) : null;
    const allowCloseWhenDownloading = Boolean(progress && progress.status === "downloading");

    const getStatusMessage = () => {
        if (checkingForUpdates) return "Checking for updates...";

        if (progress) {
            switch (progress.status) {
                case "checking":
                    return progress.totalChampions === 0
                        ? "Preparing champion data..."
                        : "Checking for updates...";
                case "downloading":
                    return "Downloading updates...";
                case "processing":
                    return `Processing ${progress.currentChampion}...`;
                default:
                    return "Processing updates...";
            }
        }

        if (isBusy) return "Updating data...";

        if (updateResult) {
            if (updateResult.error) {
                return updateResult.error;
            }

            if (updateResult.updatedChampions?.length) {
                return "Updates available";
            }

            if (updateResult.success) {
                return "Data is up to date";
            }

            return "Update status unknown";
        }

        return "Ready";
    };

    // Parse updated skin names for champions
    const [updatedSkins, setUpdatedSkins] = useState<string[]>([]);
    useEffect(() => {
        async function fetchUpdatedSkins() {
            if (!updateResult?.updatedChampions || updateResult.updatedChampions.length === 0) {
                setUpdatedSkins([]);
                return;
            }
            const manifest = await getLolSkinsManifest();
            if (!manifest || !manifest.champions) {
                setUpdatedSkins([]);
                return;
            }
            const skinNames: string[] = [];
            for (const champName of updateResult.updatedChampions) {
                const champ = manifest.champions.find(c => (c.name?.toLowerCase() === champName.toLowerCase() || c.key?.toLowerCase() === champName.toLowerCase()));
                if (champ && champ.assets?.skins) {
                    for (const skin of champ.assets.skins) {
                        if (skin.name) skinNames.push(`${champ.name ?? champ.key}: ${skin.name}`);
                    }
                }
            }
            setUpdatedSkins(skinNames);
        }
        fetchUpdatedSkins();
    }, [updateResult]);

    const items = updateResult?.updatedChampions && updateResult.updatedChampions.length > 0 ? updateResult.updatedChampions : null;
    const primaryAction = items ? { label: isBusy ? "Updating..." : "Update Now", onClick: pullUpdates, disabled: isBusy } : undefined;
    const secondaryAction = { label: isReinstalling ? "Reinstalling..." : "Reinstall Data", onClick: handleReinstall, disabled: checkingForUpdates || modalBusy };

    // Build pill meta: include manifest generated date and the number (or names) of updated champions
    const formatDate = (iso?: string | null) => {
        if (!iso) return null;
        try {
            const d = new Date(iso);
            return d.toLocaleString();
        } catch {
            return iso;
        }
    };

    const updatedCount = updateResult?.updatedChampions?.length ?? 0;
    const champListPreview = updateResult?.updatedChampions && updateResult.updatedChampions.length > 0 ? updateResult.updatedChampions.slice(0, 5).join(", ") : null;
    const pillMetaParts: string[] = [];
    const formattedDate = formatDate(manifestGeneratedAt);
    if (formattedDate) pillMetaParts.push(formattedDate);
    if (updatedCount > 0) pillMetaParts.push(`${updatedCount} champion${updatedCount > 1 ? "s" : ""}`);
    if (champListPreview) pillMetaParts.push(champListPreview);
    const pillMeta = pillMetaParts.length > 0 ? pillMetaParts.join(" • ") : null;

    const upToDateBadge = updateResult && updateResult.success && updatedCount === 0 ? "Up to date" : null;
    const pill = {
        label: shortCommit ?? "-",
        sub: undefined,
        loading: modalBusy || checkingForUpdates,
        badge: upToDateBadge ?? undefined,
        badgeVariant: upToDateBadge ? "secondary" as const : undefined,
    };

    return (
        <UpdateModal
            isOpen={isOpen}
            title={"Data Updates"}
            statusMessage={getStatusMessage()}
            isBusy={modalBusy}
            progress={progress ? { value: progress.progress, processedChampions: progress.processedChampions, totalChampions: progress.totalChampions, currentChampion: progress.currentChampion } : null}
            items={items}
            updatedSkins={updatedSkins}
            commit={manifestCommit}
            pill={pill}
            pillMeta={pillMeta}
            tertiaryAction={{ label: "Refresh", onClick: checkForUpdates, disabled: checkingForUpdates || modalBusy }}
            commitRepo={manifestRepo}
            primaryAction={primaryAction}
            secondaryAction={secondaryAction}
            onClose={() => {
                if (modalBusy && !allowCloseWhenDownloading) return;
                onClose();
            }}
        />
    );
}
