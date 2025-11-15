
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { DataUpdateProgress } from "@/lib/types";
import { useI18n } from "@/lib/i18n";

interface DataUpdateModalProps {
    isOpen: boolean;
    progress: DataUpdateProgress | null;
}

export function DataUpdateModal({ isOpen, progress }: DataUpdateModalProps) {
    const { t } = useI18n();
    const championProgress = progress && progress.totalChampions > 0 ? progress : null;
    const statusMessage = (() => {
        if (!progress) {
            return t("loading.champions_data");
        }
        switch (progress.status) {
            case "checking":
                return progress.totalChampions === 0
                    ? t("loading.champions_data")
                    : t("update.checking");
            case "downloading":
                return t("update.downloading");
            case "processing":
                if (progress.currentSkin) {
                    // Champion — Skin
                    return `${progress.currentChampion || ""} — ${progress.currentSkin}`;
                }
                return t("update.processing").replace("{champion}", progress.currentChampion || "");
            default:
                return t("update.processing_unknown");
        }
    })();

    return (
        <Dialog open={isOpen}>
            <DialogContent className="sm:max-w-md">
                <div className="flex flex-col space-y-4">
                    <DialogHeader>
                        <DialogTitle>{t("loading")}</DialogTitle>
                        <p className="text-sm text-muted-foreground">
                            {statusMessage}
                        </p>
                    </DialogHeader>
                    {championProgress && (
                        <div className="space-y-2">
                            <Progress value={championProgress.progress} />
                            <div className="flex items-center justify-between text-xs text-muted-foreground">
                                <span>
                                    {championProgress.currentChampion}
                                    {championProgress.currentSkin ? ` — ${championProgress.currentSkin}` : ""}
                                </span>
                                <span>
                                    {championProgress.processedChampions} of {championProgress.totalChampions} champions
                                </span>
                            </div>
                        </div>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    );
}
