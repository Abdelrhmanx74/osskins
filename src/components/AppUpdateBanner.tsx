"use client";

import { useEffect, useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";
import { Loader2, Download, RefreshCw } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";

export function AppUpdateBanner() {
  type UpdateEvent =
    | { event: "Started"; data?: { contentLength?: number } }
    | { event: "Progress"; data?: { chunkLength?: number } }
    | { event: "Finished" }
    | { event: string; data?: unknown };

  type UpdaterUpdate = {
    version?: string;
    downloadAndInstall?: (cb?: (event: UpdateEvent) => void) => Promise<void>;
  };

  const [updateAvailable, setUpdateAvailable] = useState<UpdaterUpdate | null>(null);
  const [checking, setChecking] = useState(true);
  const [downloading, setDownloading] = useState(false);
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    const checkForUpdates = async () => {
      try {
        const update = await check();
        // The `check` return can be null; the `available` property is deprecated and always `true`
        // so check `update` for null instead
        if (update) {
          setUpdateAvailable(update);
          toast.info(`Update available: ${update.version}`);
        }
      } catch (error) {
        console.error("Failed to check for updates:", error);
      } finally {
        setChecking(false);
      }
    };

    void checkForUpdates();
  }, []);

  const handleUpdate = async () => {
    if (!updateAvailable) return;

    setDownloading(true);
    try {
      const downloader = updateAvailable.downloadAndInstall;
      if (!downloader) return;

      const toNumber = (v: unknown) => (typeof v === "number" ? v : 0);

      await downloader((event: UpdateEvent) => {
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
            // setDownloading(false); // Keep it open until relaunch
            break;
        }
      });

      toast.success("Update installed. Restarting...");
      await relaunch();
    } catch (error) {
      console.error("Failed to update:", error);
      toast.error("Failed to update");
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


