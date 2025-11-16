import { useCallback, useEffect, useState, useTransition, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getLolSkinsManifest, getLolSkinsManifestCommit } from "@/lib/data-utils";
import { toast } from "sonner";
import type { DataUpdateProgress, DataUpdateResult } from "@/lib/types";

export interface UseDownloadingModalProps {
    isOpen: boolean;
    progress: DataUpdateProgress | null;
    onUpdateData: () => Promise<void>;
    onReinstallData: () => Promise<void>;
    isUpdating: boolean;
}

export function useDownloadingModal({
    isOpen,
    progress,
    onUpdateData,
    onReinstallData,
    isUpdating,
}: UseDownloadingModalProps) {
    const [isPending, startTransition] = useTransition();
    const [updateResult, setUpdateResult] = useState<DataUpdateResult | null>(null);
    const [checkingForUpdates, setCheckingForUpdates] = useState(false);
    const [updatingData, setUpdatingData] = useState(false);
    const [isReinstalling, setIsReinstalling] = useState(false);
    const [manifestCommit, setManifestCommit] = useState<string | null>(null);
    const [manifestRepo, setManifestRepo] = useState<string | null>(null);
    const [manifestGeneratedAt, setManifestGeneratedAt] = useState<string | null>(null);
    const isBusy = isUpdating || updatingData || isReinstalling;
    const modalBusy = isBusy;

    useEffect(() => {
        if (isOpen) {
            checkForUpdates();
            void (async () => {
                try {
                    const manifest = await getLolSkinsManifest();
                    if (manifest) {
                        const commit = getLolSkinsManifestCommit(manifest);
                        setManifestCommit(commit);
                        try {
                            const repoParts = manifest.source_repo.split("@");
                            setManifestRepo(repoParts[0]);
                        } catch { }
                        setManifestGeneratedAt(manifest.generated_at || null);
                    }
                } catch (err) {
                    console.debug("Failed to load manifest for commit info:", err);
                }
            })();
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [isOpen]);

    const checkRequestId = useRef(0);
    const checkForUpdates = useCallback(() => {
        const id = ++checkRequestId.current;
        setCheckingForUpdates(true);
        startTransition(async () => {
            let timerId: number | null = null;
            try {
                const p = invoke<DataUpdateResult>("check_data_updates");
                const timeoutPromise = new Promise<never>((_res, rej) => {
                    timerId = window.setTimeout(() => { rej(new Error("timeout")); }, 15000);
                });
                const result = await Promise.race([p, timeoutPromise]);
                if (id !== checkRequestId.current) return;
                setUpdateResult(result);
            } catch (error) {
                if ((error as Error).message === "timeout") {
                    toast.error("Data update check timed out — network may be slow or blocked");
                } else {
                    toast.error("Failed to check for updates");
                }
            } finally {
                if (typeof timerId === "number") {
                    clearTimeout(timerId);
                }
                if (id === checkRequestId.current) setCheckingForUpdates(false);
            }
        });
    }, []);

    const pullUpdates = useCallback(() => {
        setUpdatingData(true);
        startTransition(async () => {
            try {
                const updateToast = toast.loading("Updating champion data...");
                await onUpdateData();
                toast.dismiss(updateToast);
                toast.success("Data update triggered successfully");
                checkForUpdates();
            } catch (error) {
                toast.error(`Failed to update data: ${String(error)}`);
            } finally {
                setUpdatingData(false);
            }
        });
    }, [onUpdateData, checkForUpdates]);

    const handleReinstall = useCallback(() => {
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
                toast.error(`Failed to reinstall data: ${String(error)}`);
            } finally {
                setIsReinstalling(false);
            }
        });
    }, [onReinstallData, checkForUpdates]);

    const shortCommit = manifestCommit ? manifestCommit.slice(0, 8) : null;
    const allowCloseWhenDownloading = Boolean(progress && progress.status === "downloading");

    const getStatusMessage = useCallback(() => {
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
    }, [checkingForUpdates, progress, isBusy, updateResult]);

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
                const champ = manifest.champions.find(c => (c.name?.toLowerCase() === champName.toLowerCase() || c.key.toLowerCase() === champName.toLowerCase()));
                if (champ && champ.assets.skins.length > 0) {
                    for (const skin of champ.assets.skins) {
                        if (skin.name) skinNames.push(`${champ.name ?? champ.key}: ${skin.name}`);
                    }
                }
            }
            setUpdatedSkins(skinNames);
        }
        void fetchUpdatedSkins();
    }, [updateResult]);

    const items = updateResult?.updatedChampions && updateResult.updatedChampions.length > 0 ? updateResult.updatedChampions : null;
    const primaryAction = items ? { label: isBusy ? "Updating..." : "Update Now", onClick: pullUpdates, disabled: isBusy } : undefined;
    const secondaryAction = { label: isReinstalling ? "Reinstalling..." : "Reinstall Data", onClick: handleReinstall, disabled: checkingForUpdates || modalBusy };

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

    return {
        isBusy,
        modalBusy,
        updateResult,
        checkingForUpdates,
        updatingData,
        isReinstalling,
        manifestCommit,
        manifestRepo,
        manifestGeneratedAt,
        shortCommit,
        allowCloseWhenDownloading,
        getStatusMessage,
        updatedSkins,
        items,
        primaryAction,
        secondaryAction,
        pill,
        pillMeta,
        checkForUpdates,
        pullUpdates,
        handleReinstall,
    };
}
