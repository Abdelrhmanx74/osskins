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
import { toast } from "sonner";
import { Settings } from "lucide-react";
import { DropdownMenuItem } from "./ui/dropdown-menu";
import { Label } from "./ui/label";
import { ThemeToneSelector } from "./ThemeToneSelector";
import { Separator } from "./ui/separator";
import { useLeagueDirectory } from "@/lib/hooks/use-league-directory";
import { Switch } from "@/components/ui/switch";

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const leaguePath = useGameStore((s) => s.leaguePath);
  const setLeaguePath = useGameStore((s) => s.setLeaguePath);
  const autoUpdateData = useGameStore((s) => s.autoUpdateData);
  const setAutoUpdateData = useGameStore((s) => s.setAutoUpdateData);
  const { isLoading, handleSelectDirectory, handleAutoDetect } =
    useLeagueDirectory(setLeaguePath);

  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogTrigger asChild>
        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault();
            setIsOpen(true);
          }}
        >
          <Settings className="size-4" />
          Settings
        </DropdownMenuItem>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>Manage your settings.</DialogDescription>
        </DialogHeader>
        {/* Auto-update switch */}
        <div className="flex items-center justify-between">
          <Label>Auto Update Champion Data</Label>
          <Switch
            checked={autoUpdateData}
            onCheckedChange={setAutoUpdateData}
          />
        </div>

        <Separator />

        <ThemeToneSelector />

        <Separator />

        <div className="grid gap-4">
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
