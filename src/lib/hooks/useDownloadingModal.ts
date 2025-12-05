import { useCallback, useEffect, useState, useTransition, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DataUpdateProgress, DataUpdateResult } from "@/lib/types";

// New LeagueSkins repository info
const LEAGUE_SKINS_REPO = "Alban1911/LeagueSkins";
const GITHUB_API_URL = `https://api.github.com/repos/${LEAGUE_SKINS_REPO}/commits/main`;

export interface UseDownloadingModalProps {
    isOpen: boolean;
    progress: DataUpdateProgress | null;
    onUpdateData: (championsToUpdate?: string[]) => Promise<void>;
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
    const [latestCommit, setLatestCommit] = useState<string | null>(null);
    const [commitDate, setCommitDate] = useState<string | null>(null);
    const [displayCommit, setDisplayCommit] = useState<string | null>(null);
    const isBusy = isUpdating || updatingData || isReinstalling;
    const modalBusy = isBusy;

    // Fetch latest commit info from LeagueSkins repo
    useEffect(() => {
        if (isOpen) {
            checkForUpdates();
            void (async () => {
                try {
                    const response = await fetch(GITHUB_API_URL, {
                        headers: { Accept: "application/vnd.github.v3+json" },
                        cache: "no-store",
                    });
                    if (response.ok) {
                        const data = await response.json() as {
                            sha: string;
                            commit: {
                                committer: { date: string };
                                message: string;
                            };
                        };
                        setLatestCommit(data.sha);
                        setDisplayCommit(data.sha);
                        setCommitDate(data.commit.committer.date);
                    }
                } catch (err) {
                    console.debug("Failed to fetch latest commit:", err);
                }
            })();
        }
    }, [isOpen]); const checkRequestId = useRef(0);
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
                console.error("Update check failed:", error);
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
                // Pass the list of champions that need updating from check_data_updates
                const championsToUpdate = updateResult?.updatedChampions;
                console.log("[Update] Pulling updates for champions:", championsToUpdate);
                await onUpdateData(championsToUpdate);
                checkForUpdates();
            } catch (error) {
                console.error("Update data failed:", error);
            } finally {
                setUpdatingData(false);
            }
        });
    }, [onUpdateData, checkForUpdates, updateResult]);

    const handleReinstall = useCallback(() => {
        setIsReinstalling(true);
        startTransition(async () => {
            try {
                await onReinstallData();
                checkForUpdates();
            } catch (error) {
                console.error("Reinstall failed:", error);
            } finally {
                setIsReinstalling(false);
            }
        });
    }, [onReinstallData, checkForUpdates]);

    const shortCommit = latestCommit ? latestCommit.slice(0, 8) : null;
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

    // Updated skins list - now based on champion IDs from the new repo
    const [updatedSkins, setUpdatedSkins] = useState<string[]>([]);
    useEffect(() => {
        if (!updateResult?.updatedChampions || updateResult.updatedChampions.length === 0) {
            setUpdatedSkins([]);
            return;
        }
        // For the new ID-based system, we just show champion IDs/names
        // The actual skin details would come from the local champion data
        setUpdatedSkins(updateResult.updatedChampions.map(c => `Champion ${c}`));
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
    const formattedDate = formatDate(commitDate);
    if (formattedDate) pillMetaParts.push(formattedDate);
    if (updatedCount > 0) pillMetaParts.push(`${updatedCount} champion${updatedCount > 1 ? "s" : ""}`);
    if (champListPreview) pillMetaParts.push(champListPreview);
    const pillMeta = pillMetaParts.length > 0 ? pillMetaParts.join(" • ") : null;

    const upToDateBadge = updateResult && updateResult.success && updatedCount === 0 ? "Up to date" : null;
    const pill = {
        label: (displayCommit ? displayCommit.slice(0, 8) : shortCommit) ?? "-",
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
        manifestCommit: latestCommit,
        manifestRepo: LEAGUE_SKINS_REPO,
        manifestGeneratedAt: commitDate,
        shortCommit,
        displayCommit,
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
