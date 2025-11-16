"use client";

import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { ExternalLink, Tag, Calendar, GitCommit } from "lucide-react";

interface Release {
  tag_name: string;
  name: string;
  body: string;
  published_at: string;
  html_url: string;
  prerelease: boolean;
}

interface ReleaseNotesDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentVersion?: string;
}

export function ReleaseNotesDialog({
  open,
  onOpenChange,
  currentVersion,
}: ReleaseNotesDialogProps) {
  const [releases, setReleases] = useState<Release[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      fetchReleases();
    }
  }, [open]);

  const fetchReleases = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(
        "https://api.github.com/repos/Abdelrhmanx74/osskins/releases?per_page=10"
      );
      
      if (!response.ok) {
        throw new Error("Failed to fetch releases");
      }

      const data = await response.json();
      setReleases(data);
    } catch (err) {
      console.error("Error fetching releases:", err);
      setError(err instanceof Error ? err.message : "Failed to load release notes");
    } finally {
      setIsLoading(false);
    }
  };

  const formatDate = (dateStr: string) => {
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

  const isCurrentVersion = (tagName: string) => {
    if (!currentVersion) return false;
    return tagName === `v${currentVersion}` || tagName === currentVersion;
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[80vh]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <GitCommit className="h-5 w-5" />
            Release Notes
          </DialogTitle>
          <DialogDescription>
            View release history and changelog for Osskins
          </DialogDescription>
        </DialogHeader>

        <ScrollArea className="max-h-[60vh] pr-4">
          {isLoading && (
            <div className="flex items-center justify-center py-8">
              <div className="text-sm text-muted-foreground">Loading releases...</div>
            </div>
          )}

          {error && (
            <div className="flex flex-col items-center justify-center py-8 space-y-2">
              <p className="text-sm text-destructive">{error}</p>
              <Button variant="outline" size="sm" onClick={fetchReleases}>
                Retry
              </Button>
            </div>
          )}

          {!isLoading && !error && releases.length === 0 && (
            <div className="flex items-center justify-center py-8">
              <p className="text-sm text-muted-foreground">No releases found</p>
            </div>
          )}

          {!isLoading && !error && releases.length > 0 && (
            <div className="space-y-6">
              {releases.map((release) => (
                <div key={release.tag_name} className="space-y-3">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge variant={isCurrentVersion(release.tag_name) ? "default" : "outline"}>
                        <Tag className="h-3 w-3 mr-1" />
                        {release.tag_name}
                      </Badge>
                      {isCurrentVersion(release.tag_name) && (
                        <Badge variant="secondary">Current</Badge>
                      )}
                      {release.prerelease && (
                        <Badge variant="secondary">Pre-release</Badge>
                      )}
                    </div>
                    <a
                      href={release.html_url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-muted-foreground hover:text-primary transition-colors"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  </div>

                  <div>
                    <h3 className="font-semibold text-base">{release.name}</h3>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground mt-1">
                      <Calendar className="h-3 w-3" />
                      <span>{formatDate(release.published_at)}</span>
                    </div>
                  </div>

                  {release.body && (
                    <div className="text-sm text-muted-foreground bg-muted/50 rounded-md p-3">
                      <pre className="whitespace-pre-wrap font-sans leading-relaxed">
                        {release.body}
                      </pre>
                    </div>
                  )}

                  <Separator />
                </div>
              ))}
            </div>
          )}
        </ScrollArea>

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Close</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
