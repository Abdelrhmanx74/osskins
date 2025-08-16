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
import { useDataUpdate } from "@/lib/hooks/use-data-update";
import { useI18n } from "@/lib/i18n";

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(true);
  const { leaguePath, setLeaguePath } = useGameStore();
  const { updateData } = useDataUpdate();
  const { locale, setLocale, t } = useI18n();

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
      toast.success(t("settings.saved"));
    } catch (e) {
      console.error("Failed to save auto update setting", e);
      toast.error(t("settings.save_failed"));
    }
  };

  const handleSelectDirectory = async () => {
    try {
      setIsLoading(true);
      const path = await invoke<string>("select_league_directory");
      if (path) {
        setLeaguePath(path);
        toast.success(t("select.dir.success"));
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
        toast.success(t("detect.success"));
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

          {/* Auto update toggle */}
          <div className="flex items-center justify-between mt-2">
            <div className="flex flex-col">
              <Label>{t("autoUpdate.label")}</Label>
              <span className="text-xs text-muted-foreground">
                {t("autoUpdate.desc")}
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
                        toast(t("update.available"), {
                          action: {
                            label: t("update.action"),
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
