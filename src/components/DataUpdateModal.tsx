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
  const getStatusMessage = () => {
    if (!progress) return t("update.checking");

    switch (progress.status) {
      case "checking":
        return t("update.checking");
      case "downloading":
        return t("update.downloading");
      case "processing":
        return t("update.processing").replace(
          "{champion}",
          progress.currentChampion || ""
        );
      default:
        return t("update.processing_unknown");
    }
  };

  return (
    <Dialog open={isOpen}>
      <DialogContent className="sm:max-w-md">
        <div className="flex flex-col space-y-4">
          <DialogHeader>
            <DialogTitle>{t("loading")}</DialogTitle>
            <p className="text-sm text-muted-foreground">
              {getStatusMessage()}
            </p>
          </DialogHeader>
          {progress && (
            <div className="space-y-2">
              <Progress value={progress.progress} />
              <p className="text-xs text-muted-foreground text-right">
                {progress.processedChampions} of {progress.totalChampions}{" "}
                champions processed
              </p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
