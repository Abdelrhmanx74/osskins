"use client";

import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { Button } from "@/components/ui/button";
import React, { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { Badge } from "../ui/badge";
import { openPath } from "@tauri-apps/plugin-opener";
import { appDataDir, join } from "@tauri-apps/api/path";

interface UpdateModalProps {
  isOpen: boolean;
  title?: string;
  statusMessage?: string;
  isBusy?: boolean;
  progress?: {
    value: number;
    processedChampions?: number;
    totalChampions?: number;
    currentChampion?: string;
  } | null;
  items?: string[] | null;
  updatedSkins?: string[];
  commit?: string | null;
  // optional repository string in the form "owner/repo" or "org/repo" when commit is a SHA
  commitRepo?: string | null;
  // optional generic pill content to display instead of commit SHA/title
  pill?: {
    label: string;
    sub?: string;
    loading?: boolean;
    badge?: string;
    badgeVariant?: "secondary" | "destructive" | "default";
  } | null;
  // additional small metadata text to display next to the pill (e.g. date, count)
  pillMeta?: string | null;
  primaryAction?: { label: string; onClick: () => void; disabled?: boolean };
  secondaryAction?: { label: string; onClick: () => void; disabled?: boolean };
  tertiaryAction?: { label: string; onClick: () => void; disabled?: boolean };
  onClose?: () => void;
  children?: React.ReactNode;
  openFolderPath?: string; // relative to app data dir
}

export default function UpdateModal({
  isOpen,
  title = "Update",
  statusMessage,
  isBusy = false,
  progress = null,
  items = null,
  updatedSkins = [],
  commit = null,
  commitRepo = null,
  pill = null,
  pillMeta = null,
  primaryAction,
  secondaryAction,
  tertiaryAction,
  onClose,
  children,
  openFolderPath,
}: UpdateModalProps) {
  const showProgress = progress !== null && typeof progress.value === "number";
  const shortCommit = commit ? commit.slice(0, 8) : null;
  const [commitTitle, setCommitTitle] = useState<string | null>(null);
  const [loadingCommitTitle, setLoadingCommitTitle] = useState(false);

  const handleOpenFolder = async () => {
    if (!openFolderPath) return;
    try {
      const dir = await appDataDir();
      const fullPath = await join(dir, openFolderPath);
      await openPath(fullPath);
    } catch (error) {
      console.error("Failed to open folder:", error);
    }
  };

  useEffect(() => {
    let mounted = true;
    // Abort and timeout protection: avoid leaving the loading state forever
    // if the network is down or the GitHub API is unreachable.
    async function fetchCommitTitle() {
      setCommitTitle(null);
      if (!commit || !commitRepo) return;
      setLoadingCommitTitle(true);
      const controller = new AbortController();
      const timeout = setTimeout(() => {
        controller.abort();
      }, 8000);
      try {
        const repo = commitRepo;
        const sha = commit;
        const url = `https://api.github.com/repos/${repo}/commits/${sha}`;
        const res = await fetch(url, {
          headers: { Accept: "application/vnd.github.v3+json" },
          signal: controller.signal,
        });
        if (!mounted) return;
        if (!res.ok) {
          // don't spam console on 404s, just fallback to sha
          return;
        }
        const data: unknown = await res.json();
        if (typeof data === "object" && data !== null) {
          const obj = data as Record<string, unknown>;
          const commitObj = obj["commit"] as
            | Record<string, unknown>
            | undefined;
          const msg =
            commitObj && typeof commitObj["message"] === "string"
              ? commitObj["message"]
              : undefined;
          if (msg) {
            const firstLine = msg.split("\n")[0];
            setCommitTitle(firstLine);
          }
        }
      } catch (e) {
        // ignore - fallback to sha; abort will land here when timed out
      } finally {
        clearTimeout(timeout);
        if (mounted) setLoadingCommitTitle(false);
      }
    }

    if (commit && commitRepo) {
      void fetchCommitTitle();
    }

    return () => {
      mounted = false;
    };
  }, [commit, commitRepo]);

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(v: boolean) => {
        if (!v && onClose) onClose();
      }}
    >
      <DialogContent className="sm:max-w-xl">
        <div className="size-full flex flex-col space-y-4">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
            {statusMessage && (
              <p className="text-sm text-muted-foreground">{statusMessage}</p>
            )}
          </DialogHeader>

          {showProgress && (
            <div className="space-y-2">
              <Progress value={progress.value} className="transition-all" />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>{Math.round(progress.value)}%</span>
                <span>
                  {progress.processedChampions ?? 0} of{" "}
                  {progress.totalChampions ?? 0} champions
                </span>
              </div>
              {progress.currentChampion && (
                <p className="text-xs text-muted-foreground text-right">
                  Currently: {progress.currentChampion}
                </p>
              )}
            </div>
          )}

          {updatedSkins.length > 0 && (
            <div className="rounded-md bg-muted/50 p-3 border border-border">
              <p className="text-sm font-medium mb-1">Skins to be updated:</p>
              <ul className="grid gap-1 pl-4 list-disc">
                {updatedSkins.map((skin) => (
                  <li key={skin} className="text-xs text-foreground">
                    {skin}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {items && items.length > 0 && (
            <div className="rounded-md bg-muted/50 p-3 border border-border overflow-y-auto max-h-30">
              <p className="text-sm font-medium mb-1">Update Available</p>
              <p className="text-xs text-muted-foreground mb-3">
                New data updates are available for download.
              </p>
              <div className="mb-3 space-y-1 text-xs text-muted-foreground">
                <p className="font-medium text-foreground">
                  Pending champions:
                </p>
                <ul className="grid gap-1 pl-4 list-disc">
                  {items.map((it) => (
                    <li key={it}>{it}</li>
                  ))}
                </ul>
              </div>
            </div>
          )}

          {(isBusy || loadingCommitTitle || commit != null || pill != null) && (
            <div className=" flex items-center gap-3">
              <div className="w-full rounded-full bg-muted/30 px-3 py-2 border border-border text-sm flex items-center gap-3">
                {(isBusy || loadingCommitTitle || (pill?.loading ?? false)) && (
                  <Loader2 className="w-4 h-4 animate-spin text-muted-foreground" />
                )}
                <div className="flex flex-col leading-tight">
                  <span className="font-medium text-foreground line-clamp-1">
                    {commitTitle ??
                      pill?.label ??
                      shortCommit ??
                      (isBusy ? "Updating..." : "-")}
                  </span>
                  <span className="text-xs text-muted-foreground line-clamp-1">
                    {pill?.sub ?? pillMeta ?? (commitTitle ? "Commit" : "SHA")}
                  </span>
                </div>
                {pill?.badge && (
                  <div className="ml-auto">
                    <Badge variant={pill.badgeVariant ?? "secondary"}>
                      {pill.badge}
                    </Badge>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="mt-4">
          <div className="flex gap-2 w-full">
            {tertiaryAction && (
              <Button
                variant="ghost"
                size="sm"
                onClick={tertiaryAction.onClick}
                disabled={(tertiaryAction.disabled ?? false) || isBusy}
              >
                {tertiaryAction.label}
              </Button>
            )}
            {secondaryAction && (
              <Button
                variant="outline"
                size="sm"
                onClick={secondaryAction.onClick}
                disabled={(secondaryAction.disabled ?? false) || isBusy}
              >
                {secondaryAction.label}
              </Button>
            )}
            {primaryAction && (
              <Button
                size="sm"
                onClick={primaryAction.onClick}
                disabled={(primaryAction.disabled ?? false) || isBusy}
              >
                {primaryAction.label}
              </Button>
            )}
            {openFolderPath && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleOpenFolder}
                disabled={isBusy}
              >
                Open Folder
              </Button>
            )}
            <Button
              className="ml-auto"
              variant="outline"
              size="sm"
              onClick={() => {
                if (onClose) onClose();
              }}
              disabled={isBusy}
            >
              Close
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
