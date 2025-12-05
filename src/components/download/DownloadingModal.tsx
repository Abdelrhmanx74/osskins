"use client";

import UpdateModal from "./UpdateModal";
import type { DataUpdateProgress } from "@/lib/types";
import { useDownloadingModal } from "@/lib/hooks/useDownloadingModal";

export interface DownloadingModalProps {
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
    const modal = useDownloadingModal({
        isOpen,
        progress,
        onUpdateData,
        onReinstallData,
        isUpdating,
    });

    return (
        <UpdateModal
            isOpen={isOpen}
            title={"Data Updates"}
            statusMessage={modal.getStatusMessage()}
            isBusy={modal.modalBusy}
            progress={progress ? { value: progress.progress, processedChampions: progress.processedChampions, totalChampions: progress.totalChampions, currentChampion: progress.currentChampion } : null}
            items={modal.items}
            updatedSkins={modal.updatedSkins}
            // Use displayCommit if we resolved a better head commit
            commit={modal.displayCommit ?? modal.manifestCommit}
            pill={modal.pill}
            pillMeta={modal.pillMeta}
            tertiaryAction={{ label: "Refresh", onClick: modal.checkForUpdates, disabled: modal.checkingForUpdates || modal.modalBusy }}
            commitRepo={modal.manifestRepo}
            primaryAction={modal.primaryAction}
            secondaryAction={modal.secondaryAction}
            onClose={() => {
                if (modal.modalBusy && !modal.allowCloseWhenDownloading) return;
                onClose();
            }}
        />
    );
}
