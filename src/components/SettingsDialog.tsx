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
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "./ui/select";
import { ThemeToneSelector } from "./ThemeToneSelector";
import { Separator } from "./ui/separator";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/lib/i18n";

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const { leaguePath, setLeaguePath } = useGameStore();
  const { setShowUpdateModal } = useGameStore();
  const { locale, setLocale, t } = useI18n();

  // No auto-update toggle in settings UI (commit-based update logic removed)

  const handleSelectDirectory = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        setShowUpdateModal(true);
      }
    } catch (err) {
      console.error("Failed to select League directory:", err);
      toast.error(t("select.dir.failed"));
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
        setShowUpdateModal(true);
      }
    } catch (err) {
      console.error("Failed to detect League directory:", err);
      toast.error(t("detect.failed"));
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
          {t("settings.title")}
        </DropdownMenuItem>
      </DialogTrigger>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-1 gap-2">
            <Label htmlFor="leaguePath">{t("leaguePath.label")}</Label>
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
                {isLoading ? t("detecting") : t("detect.button")}
              </Button>
              <Button
                variant="outline"
                onClick={() => {
                  void handleSelectDirectory();
                }}
                disabled={isLoading}
                className="flex-1"
              >
                {t("browse.button")}
              </Button>
            </div>
          </div>

          {/* Auto-update removed: update flow is manual via TopBar */}
        </div>

        <Separator />

        {/* Language selector */}
        <div className="grid grid-cols-1 gap-2 mt-4">
          <Label>{t("language.label")}</Label>
          <Select
            value={locale}
            onValueChange={(val) => {
              const v = val as unknown as Parameters<typeof setLocale>[0];
              setLocale(v);
            }}
          >
            <SelectTrigger size="sm" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="en">English</SelectItem>
              <SelectItem value="zh">中文</SelectItem>
              <SelectItem value="ko">한국어</SelectItem>
              <SelectItem value="pt-BR">Português (Brasil)</SelectItem>
              <SelectItem value="es">Español</SelectItem>
              <SelectItem value="ru">Русский</SelectItem>
              <SelectItem value="tr">Türkçe</SelectItem>
              <SelectItem value="de">Deutsch</SelectItem>
              <SelectItem value="fr">Français</SelectItem>
              <SelectItem value="vi">Tiếng Việt</SelectItem>
              <SelectItem value="ar">العربية</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <ThemeToneSelector />

        {/* Watermark Notice */}
        <div className="text-xs text-center mt-2 select-none">
          {t("watermark")}{" "}
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
            <Button variant="default">{t("close.button")}</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
