"use client";

import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { Loader2, Download, RefreshCw } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";

type UpdateEvent =
  | { event: "Started"; data?: { contentLength?: number } }
  | { event: "Progress"; data?: { chunkLength?: number } }
  | { event: "Finished" }
  | { event: string; data?: unknown };

type UpdaterUpdate = {
  version: string;
  downloadAndInstall: (cb?: (event: UpdateEvent) => void) => Promise<void>;
};

export function AppUpdateBanner() {
  const [updateAvailable, setUpdateAvailable] = useState<UpdaterUpdate | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    // Skip updater in development - it breaks the dev server on relaunch
    if (process.env.NODE_ENV === "development") {
      return;
    }

    let cancelled = false;

    const checkForUpdates = async () => {
      // Wait a bit for Tauri to fully initialize
      await new Promise(resolve => setTimeout(resolve, 2000));

      if (cancelled) return;

      try {
        const updaterModule = await import("@tauri-apps/plugin-updater");
        if (!updaterModule?.check) return;

        const update = await updaterModule.check();

        if (cancelled) return;

        // Safely check if update exists and has required properties
        if (
          update &&
          typeof update === "object" &&
          "version" in update &&
          "downloadAndInstall" in update &&
          typeof update.version === "string" &&
          typeof update.downloadAndInstall === "function"
        ) {
          setUpdateAvailable(update as UpdaterUpdate);
          toast.info(`Update available: ${update.version}`);
        }
      } catch {
        // Silently ignore - updater errors are expected in dev mode
      }
    };

    void checkForUpdates();

    return () => { cancelled = true; };
  }, []);

  const handleUpdate = async () => {
    if (!updateAvailable) return;

    setDownloading(true);
    try {
      const toNumber = (v: unknown) => (typeof v === "number" ? v : 0);

      await updateAvailable.downloadAndInstall((event: UpdateEvent) => {
        switch (event.event) {
          case 'Started': {
            const data = event.data as { contentLength?: unknown } | undefined;
            const contentLen = toNumber(data?.contentLength);
            setTotal(contentLen);
            break;
          }
          case 'Progress': {
            const data = event.data as { chunkLength?: unknown } | undefined;
            const chunk = toNumber(data?.chunkLength);
            setDownloaded((prev: number) => prev + chunk);
            break;
          }
          case 'Finished':
            break;
        }
      });

      toast.success("Update installed. Restarting...");
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error("Failed to update:", errorMessage);
      toast.error(errorMessage || "Failed to update");
      setDownloading(false);
    }
  };

  if (!updateAvailable) return null;

  return (
    <>
      <div className="bg-primary/10 border-b border-primary/20 p-2 flex items-center justify-between px-4">
        <div className="flex items-center gap-2 text-sm">
          <RefreshCw className="h-4 w-4 animate-spin-slow" />
          <span>New version available: {updateAvailable.version}</span>
        </div>
        <Button size="sm" variant="default" onClick={() => void handleUpdate()} disabled={downloading}>
          {downloading ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4 mr-2" />}
          {downloading ? "Updating..." : "Update Now"}
        </Button>
      </div>

      <Dialog open={downloading} onOpenChange={() => { }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Updating to {updateAvailable.version}</DialogTitle>
            <DialogDescription>
              Please wait while the update is being downloaded and installed.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <Progress value={total > 0 ? (downloaded / total) * 100 : 0} />
            <p className="text-sm text-muted-foreground text-center">
              {total > 0 ? `${(downloaded / 1024 / 1024).toFixed(1)} MB / ${(total / 1024 / 1024).toFixed(1)} MB` : "Preparing..."}
            </p>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}


