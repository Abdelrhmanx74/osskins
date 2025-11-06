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
import type { DataUpdateProgress, DataUpdateResult } from "@/lib/types";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useTransition } from "react";
import { toast } from "sonner";

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
    const isBusy = isUpdating || updatingData || isReinstalling;

    // Check for GitHub updates when the modal is opened
    useEffect(() => {
        if (isOpen) {
            checkForUpdates();
        }
    }, [isOpen]);

    // Function to check for updates from GitHub
    const checkForUpdates = () => {
        setCheckingForUpdates(true);

        startTransition(async () => {
            try {
                const result = await invoke<DataUpdateResult>("check_data_updates");
                setUpdateResult(result);
                console.log("Update check result:", result);
            } catch (error) {
                console.error("Failed to check for updates:", error);
                toast.error("Failed to check for updates");
            } finally {
                setCheckingForUpdates(false);
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

    const getStatusMessage = () => {
        if (checkingForUpdates) return "Checking for updates...";

        if (progress) {
            switch (progress.status) {
                case "checking":
                    return "Checking for updates...";
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

    return (
        <Dialog
            open={isOpen}
            onOpenChange={(open) => {
                if (!open) onClose();
            }}
        >
            <DialogContent className="sm:max-w-md">
                <div className="flex flex-col space-y-4">
                    <DialogHeader>
                        <DialogTitle>Data Updates</DialogTitle>
                        <p className="text-sm text-muted-foreground animate-in fade-in-50 duration-300">
                            {getStatusMessage()}
                        </p>
                    </DialogHeader>

                    {/* Regular progress indicator for initial data download */}
                    {progress && isBusy && (
                        <div className="space-y-2 animate-in fade-in-50 duration-300">
                            <Progress
                                value={progress.progress}
                                className={`transition-all duration-300 ${isPending ? "opacity-50" : "opacity-100"}`}
                            />
                            <div className="flex justify-between text-xs text-muted-foreground">
                                <span>{Math.round(progress.progress)}%</span>
                                <span>
                                    {progress.processedChampions} of {progress.totalChampions}{" "}
                                    champions
                                </span>
                            </div>
                            <p className="text-xs text-muted-foreground text-right animate-in slide-in-from-right-5">
                                {progress.currentChampion &&
                                    `Currently processing: ${progress.currentChampion}`}
                            </p>
                        </div>
                    )}

                    {/* GitHub update UI */}
                    {updateResult && (
                        <div className="space-y-3 animate-in fade-in-50 duration-300">
                            <div className="grid grid-cols-2 gap-2 text-sm">
                                <span className="text-muted-foreground">Current Version:</span>
                                <span className="font-mono">
                                    {/* Legacy: show nothing, or fallback to success/error */}
                                    {updateResult.success ? "Up to date" : "Not installed"}
                                </span>

                                <span className="text-muted-foreground">Latest Version:</span>
                                <span className="font-mono">
                                    {/* No available_version in type, fallback to success/error */}
                                    {updateResult.success ? "Up to date" : "Unknown"}
                                </span>
                            </div>

                            {/* No has_update in type, fallback to error/success logic */}
                            {updateResult.error ||
                                (updateResult.updatedChampions &&
                                    updateResult.updatedChampions.length > 0) ? (
                                <div className="rounded-md bg-muted/50 p-3 border border-border">
                                    <p className="text-sm font-medium mb-1">Update Available</p>
                                    <p className="text-xs text-muted-foreground mb-3">
                                        {updateResult.error ??
                                            "New data updates are available for download."}
                                    </p>
                                    {updateResult.updatedChampions &&
                                        updateResult.updatedChampions.length > 0 && (
                                            <div className="mb-3 space-y-1 text-xs text-muted-foreground">
                                                <p className="font-medium text-foreground">
                                                    Pending champions:
                                                </p>
                                                <ul className="grid gap-1 pl-4 list-disc">
                                                    {updateResult.updatedChampions.map((champion) => (
                                                        <li key={champion}>{champion}</li>
                                                    ))}
                                                </ul>
                                            </div>
                                        )}
                                    <Button
                                        size="sm"
                                        className="w-full"
                                        disabled={isBusy}
                                        onClick={pullUpdates}
                                    >
                                        {isBusy ? "Updating..." : "Update Now"}
                                    </Button>
                                </div>
                            ) : (
                                <div className="rounded-md bg-muted/50 p-3 border border-border">
                                    <p className="text-sm font-medium flex items-center gap-2">
                                        <svg
                                            xmlns="http://www.w3.org/2000/svg"
                                            width="16"
                                            height="16"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="currentColor"
                                            strokeWidth="2"
                                            strokeLinecap="round"
                                            strokeLinejoin="round"
                                            className="text-green-500"
                                        >
                                            <title>Data is up to date</title>
                                            <path d="M20 6L9 17l-5-5" />
                                        </svg>
                                        Data is up to date
                                    </p>
                                    <p className="text-xs text-muted-foreground mt-1">
                                        You have the latest champion data installed.
                                    </p>
                                </div>
                            )}
                        </div>
                    )}

                    {/* Loading state */}
                    {(checkingForUpdates || isBusy) && !updateResult && !progress && (
                        <div className="flex flex-col items-center justify-center py-4">
                            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-primary mb-2" />
                            <p className="text-sm text-muted-foreground">
                                {checkingForUpdates
                                    ? "Checking for updates..."
                                    : "Updating data..."}
                            </p>
                        </div>
                    )}
                </div>

                <DialogFooter className="mt-4">
                    <Button
                        variant="destructive"
                        size="sm"
                        onClick={handleReinstall}
                        disabled={checkingForUpdates || isBusy}
                    >
                        {isReinstalling ? "Reinstalling..." : "Reinstall Data"}
                    </Button>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={checkForUpdates}
                        disabled={checkingForUpdates || isBusy}
                    >
                        Check for Updates
                    </Button>
                    <Button size="sm" onClick={onClose} disabled={isBusy}>
                        Close
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
