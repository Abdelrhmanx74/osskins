import { useState } from "react";
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

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const { leaguePath, setLeaguePath } = useGameStore();

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
