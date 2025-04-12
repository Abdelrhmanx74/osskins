import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { DataUpdateProgress } from "@/lib/types";

interface DataUpdateModalProps {
  isOpen: boolean;
  progress: DataUpdateProgress | null;
}

export function DataUpdateModal({ isOpen, progress }: DataUpdateModalProps) {
  const getStatusMessage = () => {
    if (!progress) return "Checking for updates...";

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
  };

  return (
    <Dialog open={isOpen}>
      <DialogContent className="sm:max-w-md">
        <div className="flex flex-col space-y-4">
          <DialogHeader>
            <DialogTitle>Loading...</DialogTitle>
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
