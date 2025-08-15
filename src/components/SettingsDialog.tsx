import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogTrigger,
  DialogFooter,
  DialogClose,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useGameStore } from "@/lib/store";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Settings } from "lucide-react";
import { DropdownMenuItem } from "./ui/dropdown-menu";
import { Label } from "./ui/label";
import { ThemeToneSelector } from "./ThemeToneSelector";
import { Separator } from "./ui/separator";
import { Switch } from "@/components/ui/switch";
import { useDataUpdate } from "@/lib/hooks/use-data-update";

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(true);
  const { leaguePath, setLeaguePath } = useGameStore();
  const { updateData } = useDataUpdate();

  useEffect(() => {
    // Load current setting
    const load = async () => {
      try {
        const cfg = await invoke("load_config");
        if (cfg && typeof cfg === "object" && "auto_update_data" in cfg) {
          const v = (cfg as Record<string, unknown>)["auto_update_data"];
          setAutoUpdate(v !== false);
        }
      } catch (e) {
        // ignore
      }
    };
    void load();
  }, []);

  const persistAutoUpdate = async (value: boolean) => {
    try {
      await invoke("set_auto_update_data", { value });
      toast.success("Settings saved");
    } catch (e) {
      console.error("Failed to save auto update setting", e);
      toast.error("Failed to save settings");
    }
  };

  const handleSelectDirectory = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        toast.success("League of Legends directory updated successfully");
      }
    } catch (err) {
      console.error("Failed to select League directory:", err);
      toast.error("Failed to select directory");
    } finally {
      setIsLoading(false);
    }
  };

  const handleAutoDetect = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("auto_detect_league");
      if (path) {
        setLeaguePath(path);
        toast.success("League of Legends installation found");
      }
    } catch (err) {
      console.error("Failed to detect League directory:", err);
      toast.error(
        "Could not find League of Legends installation automatically"
      );
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault();
            setIsOpen(true);
          }}
        >
          <Settings className="h-4 w-4" />
          Settings
        </DropdownMenuItem>
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Manage your League of Legends installation path and other settings.
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-1 gap-2">
            <Label htmlFor="leaguePath">League of Legends Path</Label>
            <Input
              id="leaguePath"
              value={leaguePath ?? ""}
              readOnly
              className="flex-1"
            />
            <div className="flex gap-2">
              <Button
                onClick={() => {
                  void handleAutoDetect();
                }}
                disabled={isLoading}
                className="flex-1"
                variant="secondary"
              >
                {isLoading ? "Detecting..." : "Detect"}
              </Button>
              <Button
                variant="outline"
                onClick={() => {
                  void handleSelectDirectory();
                }}
                disabled={isLoading}
                className="flex-1"
              >
                Browse
              </Button>
            </div>
          </div>

          {/* Auto update toggle */}
          <div className="flex items-center justify-between mt-2">
            <div className="flex flex-col">
              <Label>Auto update data</Label>
              <span className="text-xs text-muted-foreground">
                Automatically download new champion data
              </span>
            </div>
            <Switch
              checked={autoUpdate}
              onCheckedChange={(v) => {
                const next = !!v;
                setAutoUpdate(next);
                void persistAutoUpdate(next);
                // If turning OFF auto-update, immediately check and prompt
                if (!next) {
                  void (async () => {
                    try {
                      const info = await invoke<{
                        success: boolean;
                        updatedChampions?: string[];
                      }>("check_data_updates");
                      const hasNew = (info.updatedChampions?.length ?? 0) > 0;
                      if (hasNew) {
                        toast("New data is available", {
                          action: {
                            label: "Update data",
                            onClick: () => {
                              void updateData();
                            },
                          },
                        });
                      }
                    } catch (e) {
                      console.warn("Failed to check updates after toggle", e);
                    }
                  })();
                }
              }}
            />
          </div>
        </div>

        <Separator />

        <ThemeToneSelector />

        {/* Watermark Notice */}
        <div className="text-xs text-center mt-2 select-none">
          This app is 100% free do not buy it from anyone. Join our community at{" "}
          <a
            href="https://discord.gg/tHyHnx5DKX"
            target="_blank"
            rel="noopener noreferrer"
            className="underline"
          >
            https://discord.gg/tHyHnx5DKX
          </a>
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="default">Close</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
