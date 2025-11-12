
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
        return t("update.processing").replace(
          "{champion}",
          progress.currentChampion || "",
        );
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
              <p className="text-xs text-muted-foreground text-right">
                {championProgress.processedChampions} of {championProgress.totalChampions}{" "}
                champions processed
              </p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
