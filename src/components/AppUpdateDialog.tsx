"use client";

import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { useAppUpdaterStore } from "@/lib/store/updater";
import { useSoftUpdater } from "@/lib/hooks/use-soft-updater";
import { Download, RefreshCw, Check, AlertCircle, Sparkles } from "lucide-react";
import { Separator } from "@/components/ui/separator";

interface AppUpdateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AppUpdateDialog({ open, onOpenChange }: AppUpdateDialogProps) {
  const {
    status,
    availableVersion,
    currentVersion,
    releaseNotes,
    pubDate,
    progress,
    error,
  } = useAppUpdaterStore();
  
  const { checkForUpdates, downloadUpdate, installUpdate } = useSoftUpdater();
  const [isChecking, setIsChecking] = useState(false);

  const handleCheckForUpdates = async () => {
    setIsChecking(true);
    try {
      await checkForUpdates({ silent: false });
    } finally {
      setIsChecking(false);
    }
  };

  const handleDownload = async () => {
    await downloadUpdate();
  };

  const handleInstall = async () => {
    await installUpdate();
  };

  const formatDate = (dateStr: string | null) => {
    if (!dateStr) return "Unknown date";
    try {
      return new Date(dateStr).toLocaleDateString(undefined, {
        year: "numeric",
        month: "long",
        day: "numeric",
      });
    } catch {
      return dateStr;
    }
  };

  const renderContent = () => {
    switch (status) {
      case "checking":
        return (
          <div className="flex flex-col items-center justify-center py-8 space-y-4">
            <RefreshCw className="h-12 w-12 animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">Checking for updates...</p>
          </div>
        );

      case "available":
        return (
          <div className="space-y-4">
            <div className="flex items-start gap-3">
              <Sparkles className="h-5 w-5 text-primary mt-1" />
              <div className="flex-1">
                <h4 className="font-semibold">New version available!</h4>
                <p className="text-sm text-muted-foreground mt-1">
                  Version {availableVersion} is now available. You are currently on version {currentVersion}.
                </p>
                {pubDate && (
                  <p className="text-xs text-muted-foreground mt-2">
                    Released on {formatDate(pubDate)}
                  </p>
                )}
              </div>
            </div>

            {releaseNotes && (
              <>
                <Separator />
                <div className="space-y-2">
                  <h4 className="text-sm font-semibold">Release Notes</h4>
                  <div className="text-sm text-muted-foreground max-h-48 overflow-y-auto p-3 bg-muted/50 rounded-md">
                    <pre className="whitespace-pre-wrap font-sans">{releaseNotes}</pre>
                  </div>
                </div>
              </>
            )}
          </div>
        );

      case "downloading":
        return (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <Download className="h-5 w-5 text-primary animate-pulse" />
              <div className="flex-1">
                <h4 className="font-semibold">Downloading update...</h4>
                <p className="text-sm text-muted-foreground">
                  Please wait while the update is being downloaded.
                </p>
              </div>
            </div>
            <Progress value={progress || 0} className="w-full" />
            <p className="text-xs text-center text-muted-foreground">
              {progress || 0}% complete
            </p>
          </div>
        );

      case "downloaded":
        return (
          <div className="flex flex-col items-center justify-center py-6 space-y-4">
            <div className="rounded-full bg-green-500/20 p-3">
              <Check className="h-8 w-8 text-green-500" />
            </div>
            <div className="text-center space-y-2">
              <h4 className="font-semibold">Update downloaded successfully!</h4>
              <p className="text-sm text-muted-foreground">
                Click "Install & Restart" to complete the update.
              </p>
            </div>
          </div>
        );

      case "up-to-date":
        return (
          <div className="flex flex-col items-center justify-center py-8 space-y-4">
            <div className="rounded-full bg-green-500/20 p-3">
              <Check className="h-8 w-8 text-green-500" />
            </div>
            <div className="text-center space-y-2">
              <h4 className="font-semibold">You're up to date!</h4>
              <p className="text-sm text-muted-foreground">
                You are running the latest version ({currentVersion || "unknown"}).
              </p>
            </div>
          </div>
        );

      case "error":
        return (
          <div className="flex flex-col items-center justify-center py-6 space-y-4">
            <div className="rounded-full bg-red-500/20 p-3">
              <AlertCircle className="h-8 w-8 text-red-500" />
            </div>
            <div className="text-center space-y-2">
              <h4 className="font-semibold">Update failed</h4>
              <p className="text-sm text-muted-foreground max-w-md">
                {error || "An unknown error occurred while checking for updates."}
              </p>
            </div>
          </div>
        );

      default:
        return (
          <div className="flex flex-col items-center justify-center py-8 space-y-4">
            <RefreshCw className="h-12 w-12 text-muted-foreground" />
            <div className="text-center space-y-2">
              <h4 className="font-semibold">Check for Updates</h4>
              <p className="text-sm text-muted-foreground">
                Click the button below to check for available updates.
              </p>
            </div>
          </div>
        );
    }
  };

  const renderActions = () => {
    switch (status) {
      case "checking":
        return (
          <Button variant="outline" disabled>
            Checking...
          </Button>
        );

      case "available":
        return (
          <>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Later
            </Button>
            <Button onClick={handleDownload}>
              <Download className="h-4 w-4 mr-2" />
              Download Update
            </Button>
          </>
        );

      case "downloading":
        return (
          <Button variant="outline" disabled>
            Downloading...
          </Button>
        );

      case "downloaded":
        return (
          <>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Later
            </Button>
            <Button onClick={handleInstall}>
              <RefreshCw className="h-4 w-4 mr-2" />
              Install & Restart
            </Button>
          </>
        );

      case "up-to-date":
        return (
          <Button onClick={() => onOpenChange(false)}>
            Close
          </Button>
        );

      case "error":
        return (
          <>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Close
            </Button>
            <Button onClick={handleCheckForUpdates} disabled={isChecking}>
              <RefreshCw className="h-4 w-4 mr-2" />
              Try Again
            </Button>
          </>
        );

      default:
        return (
          <>
            <Button variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleCheckForUpdates} disabled={isChecking}>
              <RefreshCw className="h-4 w-4 mr-2" />
              Check for Updates
            </Button>
          </>
        );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>App Updates</DialogTitle>
          <DialogDescription>
            Keep your app up to date with the latest features and fixes.
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {renderContent()}
        </div>

        <DialogFooter className="flex gap-2">
          {renderActions()}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
