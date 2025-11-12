"use client";

import { useMemo } from "react";
import { Sparkles, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { useI18n } from "@/lib/i18n";
import { useAppUpdaterStore } from "@/lib/store/updater";
import { useSoftUpdater } from "@/lib/hooks/use-soft-updater";

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024)),
  );
  const value = bytes / Math.pow(1024, exponent);
  const precision = exponent === 0 ? 0 : value < 10 ? 2 : value < 100 ? 1 : 0;
  return `${value.toFixed(precision)} ${units[exponent]}`;
}

export function AppUpdateBanner() {
  const { t } = useI18n();
  const status = useAppUpdaterStore((state) => state.status);
  const availableVersion = useAppUpdaterStore((state) => state.availableVersion);
  const releaseNotes = useAppUpdaterStore((state) => state.releaseNotes);
  const progress = useAppUpdaterStore((state) => state.progress);
  const downloadedBytes = useAppUpdaterStore((state) => state.downloadedBytes);
  const totalBytes = useAppUpdaterStore((state) => state.totalBytes);
  const error = useAppUpdaterStore((state) => state.error);
  const bannerDismissed = useAppUpdaterStore((state) => state.bannerDismissed);
  const lastCheckedAt = useAppUpdaterStore((state) => state.lastCheckedAt);
  const currentVersion = useAppUpdaterStore((state) => state.currentVersion);
  const { downloadUpdate, installUpdate, dismissBanner, checkForUpdates } =
    useSoftUpdater();
  const hasUpdateHandle = useAppUpdaterStore((state) => state.updateHandle !== null);

  const shouldHide =
    status === "idle" ||
    status === "up-to-date" ||
    status === "installed" ||
    (status === "available" && bannerDismissed);

  if (shouldHide) {
    return null;
  }

  const isDownloading = status === "downloading";
  const isInstalling = status === "installing";
  const isAvailable = status === "available";
  const isDownloaded = status === "downloaded";
  const isError = status === "error";
  const isChecking = status === "checking";

  const notesPreview = useMemo(() => {
    if (!releaseNotes) {
      return null;
    }
    const cleaned = releaseNotes.replace(/\s+/g, " ").trim();
    if (cleaned.length === 0) {
      return null;
    }
    if (cleaned.length <= 280) {
      return cleaned;
    }
    return `${cleaned.slice(0, 277)}…`;
  }, [releaseNotes]);

  const progressLabel = useMemo(() => {
    if (downloadedBytes != null && totalBytes != null && totalBytes > 0) {
      return `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)} (${progress ?? 0}%)`;
    }
    if (downloadedBytes != null && !totalBytes) {
      return formatBytes(downloadedBytes);
    }
    if (progress != null) {
      return `${progress}%`;
    }
    return null;
  }, [downloadedBytes, totalBytes, progress]);

  const lastCheckedLabel = useMemo(() => {
    if (!lastCheckedAt) {
      return null;
    }
    try {
      const date = new Date(lastCheckedAt);
      return date.toLocaleString();
    } catch (e) {
      console.warn("Failed to format last checked timestamp", e);
      return null;
    }
  }, [lastCheckedAt]);

  const retryAction = useMemo(() => {
    if (hasUpdateHandle) {
      return () => downloadUpdate();
    }
    return () => checkForUpdates({ silent: false });
  }, [checkForUpdates, downloadUpdate, hasUpdateHandle]);

  return (
    <div className="border-b border-primary/30 bg-primary/10 text-sm">
      <div className="mx-auto flex w-full max-w-screen-2xl flex-col gap-3 px-4 py-3 md:flex-row md:items-center md:justify-between">
        <div className="flex flex-1 flex-col gap-2">
          <div className="flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-primary" />
            <span className="font-semibold text-sm md:text-base">
              {isDownloaded
                ? t("appUpdate.status.downloaded")
                : isDownloading
                  ? t("appUpdate.status.downloading")
                  : isInstalling
                    ? t("appUpdate.status.installing")
                    : isChecking
                      ? t("appUpdate.status.checking")
                      : isError
                        ? t("appUpdate.status.error", { message: error ?? t("loading"), })
                        : t("appUpdate.title")}
            </span>
            {availableVersion && (
              <Badge variant="outline" className="font-mono">
                v{availableVersion}
              </Badge>
            )}
          </div>
          <div className="flex flex-col gap-1 text-xs text-muted-foreground md:text-sm">
            <span>
              {currentVersion
                ? t("appUpdate.currentVersion", { version: currentVersion })
                : null}
            </span>
            {notesPreview && <span>{notesPreview}</span>}
            {lastCheckedLabel && (
              <span className="text-muted-foreground/80">
                {t("appUpdate.lastChecked", { timestamp: lastCheckedLabel })}
              </span>
            )}
          </div>
          {(isDownloading || isDownloaded) && progress != null && (
            <div className="flex max-w-lg flex-col gap-1">
              <Progress value={progress} />
              {progressLabel && (
                <span className="text-xs text-muted-foreground">
                  {progressLabel}
                </span>
              )}
            </div>
          )}
          {isError && error && (
            <span className="text-xs text-destructive md:text-sm">
              {t("appUpdate.status.error", { message: error })}
            </span>
          )}
        </div>
        <div className="flex shrink-0 flex-wrap gap-2">
          {isAvailable && (
            <>
              <Button
                size="sm"
                onClick={() => {
                  void downloadUpdate();
                }}
              >
                {t("appUpdate.actions.download")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  dismissBanner();
                }}
              >
                {t("appUpdate.actions.dismiss")}
              </Button>
            </>
          )}
          {isDownloading && (
            <Button size="sm" disabled>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              {t("appUpdate.status.downloading")}
            </Button>
          )}
          {isDownloaded && (
            <>
              <Button
                size="sm"
                onClick={() => {
                  void installUpdate();
                }}
              >
                {t("appUpdate.actions.install")}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  dismissBanner();
                }}
              >
                {t("appUpdate.actions.dismiss")}
              </Button>
            </>
          )}
          {isInstalling && (
            <Button size="sm" disabled>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              {t("appUpdate.status.installing")}
            </Button>
          )}
          {isError && (
            <>
              <Button
                size="sm"
                onClick={() => {
                  void retryAction();
                }}
              >
                {t("appUpdate.actions.retry")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  dismissBanner();
                }}
              >
                {t("appUpdate.actions.dismiss")}
              </Button>
            </>
          )}
          {isChecking && (
            <Button size="sm" disabled>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              {t("appUpdate.status.checking")}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
