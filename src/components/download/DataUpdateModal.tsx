
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { DataUpdateProgress } from "@/lib/types";
import { useI18n } from "@/lib/i18n";
import { Loader2 } from "lucide-react";

interface DataUpdateModalProps {
    isOpen: boolean;
    progress: DataUpdateProgress | null;
}

export function DataUpdateModal({ isOpen, progress }: DataUpdateModalProps) {
    const { t } = useI18n();
    const championProgress = progress && progress.totalChampions > 0 ? progress : null;

    const getStatusIcon = () => {
        return <Loader2 className="h-5 w-5 animate-spin text-primary" />;
    };

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
                if (progress.currentChampion) {
                    return progress.currentChampion;
                }
                return t("update.downloading");
            case "processing":
                if (progress.currentSkin) {
                    return (
                        <span className="flex items-center gap-2 truncate">
                            <span className="font-medium">{progress.currentChampion}</span>
                            <span className="text-muted-foreground">—</span>
                            <span className="text-muted-foreground truncate">{progress.currentSkin}</span>
                        </span>
                    );
                }
                return t("update.processing").replace("{champion}", progress.currentChampion || "");
            default:
                return t("update.processing_unknown");
        }
    })();

    return (
        <Dialog open={isOpen}>
            <DialogContent className="sm:max-w-md" onInteractOutside={(e) => { e.preventDefault(); }}>
                <div className="flex flex-col space-y-6 py-4">
                    <DialogHeader>
                        <DialogTitle className="flex items-center gap-2 text-xl">
                            {getStatusIcon()}
                            {t("loading")}
                        </DialogTitle>
                    </DialogHeader>

                    <div className="space-y-4">
                        <div className="flex items-center justify-between text-sm min-h-5">
                            <div className="text-muted-foreground truncate max-w-[300px]">
                                {statusMessage}
                            </div>
                            {championProgress && (
                                <span className="font-mono text-xs text-muted-foreground tabular-nums">
                                    {Math.round(championProgress.progress)}%
                                </span>
                            )}
                        </div>

                        {championProgress && (
                            <div className="space-y-1.5">
                                <Progress value={championProgress.progress} className="h-2" />
                                <div className="flex justify-between text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                                    <span>Processing</span>
                                    <span>
                                        {championProgress.processedChampions} / {championProgress.totalChampions}
                                    </span>
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            </DialogContent>
        </Dialog>
    );
}
