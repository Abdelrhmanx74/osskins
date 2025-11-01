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
import { Terminal, Clipboard, Check } from "lucide-react";
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
import { skinManagementApi } from "@/lib/api/skin-management";
import { Upload, Download } from "lucide-react";
import { useRef } from "react";

export function SettingsDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const { leaguePath, setLeaguePath } = useGameStore();
  const { setShowUpdateModal } = useGameStore();
  const { locale, setLocale, t } = useI18n();
  const [copied, setCopied] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const {
    clearAllSelections,
    selectSkin,
    manualInjectionMode,
    setManualInjectionMode,
  } = useGameStore();

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

  // Export only the skins array to a JSON file
  const handleExportSkins = async () => {
    try {
      const cfg = await skinManagementApi.loadConfig();
      const data = JSON.stringify(cfg.skins, null, 2);
      const blob = new Blob([data], { type: "application/json" });
      const ts = new Date();
      const pad = (n: number) => String(n).padStart(2, "0");
      const filename = `skins-export-${ts.getFullYear()}${pad(
        ts.getMonth() + 1,
      )}${pad(ts.getDate())}-${pad(ts.getHours())}${pad(ts.getMinutes())}${pad(
        ts.getSeconds(),
      )}.json`;
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      toast.success(t("export.skins.success"));
    } catch (err) {
      console.error("Export skins failed", err);
      toast.error(t("export.skins.failed"));
    }
  };

  // Import a JSON file containing an array of SkinData and save it
  const handleImportSkinsFromFile = async (file: File) => {
    try {
      const text = await file.text();
      const parsed: unknown = JSON.parse(text);
      if (!Array.isArray(parsed))
        throw new Error("Invalid format: not an array");

      // Basic shape validation and normalization
      type AnyRecord = Record<string, unknown>;
      const isRecord = (v: unknown): v is AnyRecord =>
        typeof v === "object" && v !== null;

      const skins: Array<{
        champion_id: number;
        skin_id: number;
        chroma_id?: number;
        fantome?: string;
      }> = (parsed as unknown[])
        .filter(isRecord)
        .map((it) => ({
          champion_id: Number(it["champion_id"] as number | string),
          skin_id: Number(it["skin_id"] as number | string),
          chroma_id:
            (it["chroma_id"] as number | string | undefined | null) != null
              ? Number(it["chroma_id"] as number | string)
              : undefined,
          fantome:
            typeof it["fantome"] === "string" ? it["fantome"] : undefined,
        }))
        .filter(
          (it) =>
            Number.isFinite(it.champion_id) && Number.isFinite(it.skin_id),
        );

      if (skins.length === 0) throw new Error("Empty or invalid skins array");

      // Load current config to preserve other fields
      const cfg = await skinManagementApi.loadConfig();
      await skinManagementApi.saveSelectedSkins(
        cfg.league_path ?? "",
        skins,
        cfg.favorites,
        cfg.theme,
        cfg.selected_misc_items,
      );

      // Update local UI selections for immediate feedback
      clearAllSelections();
      for (const s of skins) {
        selectSkin(s.champion_id, s.skin_id, s.chroma_id, s.fantome);
      }

      toast.success(t("import.skins.success", { count: skins.length }));
    } catch (err) {
      console.error("Import skins failed", err);
      toast.error(t("import.skins.failed"));
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
      <DialogContent className="sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogDescription>{t("settings.description")}</DialogDescription>
        </DialogHeader>

        {/* Split horizontally */}
        <div
          className="flex flex-row gap-6 py-4"
          style={{ minHeight: 400, maxHeight: "70vh", overflowY: "auto" }}
        >
          {/* Left Side */}
          <div className="flex-1 min-w-0 flex flex-col gap-4">
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

            <Separator />

            {/* Import / Export skins */}
            <div className="grid grid-cols-1 gap-3 mt-4">
              <Label>{t("skins.import_export.title")}</Label>
              <div className="flex gap-2">
                <Button
                  variant="secondary"
                  className="flex-1"
                  onClick={() => {
                    void handleExportSkins();
                  }}
                >
                  <Download className="h-4 w-4 mr-2" />
                  {t("export.skins")}
                </Button>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="application/json,.json"
                  className="hidden"
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) void handleImportSkinsFromFile(f);
                    if (fileInputRef.current) fileInputRef.current.value = "";
                  }}
                />
                <Button
                  variant="outline"
                  className="flex-1"
                  onClick={() => fileInputRef.current?.click()}
                >
                  <Upload className="h-4 w-4 mr-2" />
                  {t("import.skins")}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {t("skins.import_export.help")}
              </p>
            </div>
            {/* Auto-update removed: update flow is manual via TopBar */}
            <Separator />

            {/* Manual Injection Mode Toggle */}
            <div className="grid grid-cols-1 gap-2 mt-4">
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="manual-injection-mode">
                    {t("settings.manual_injection_mode")}
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    {t("settings.manual_injection_mode_description")}
                  </p>
                </div>
                <Switch
                  id="manual-injection-mode"
                  checked={manualInjectionMode}
                  onCheckedChange={setManualInjectionMode}
                />
              </div>
            </div>
          </div>

          <Separator orientation="vertical" className="mx-2" />

          {/* Right Side */}
          <div className="flex-1 min-w-0 flex flex-col gap-4">
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

            <Separator />

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
          </div>
        </div>

        <DialogFooter className="w-full flex sm:justify-between items-center">
          <Button
            variant="outline"
            onClick={() => {
              void (async () => {
                try {
                  const path = await invoke<string>("print_logs");
                  // Signal copied/available
                  setCopied(true);
                  toast.success(`${t("logs.saved")} ${path}`);
                  setTimeout(() => {
                    setCopied(false);
                  }, 2000);
                } catch (e) {
                  console.error(e);
                  toast.error(t("logs.failed"));
                }
              })();
            }}
          >
            {copied ? (
              <div className="flex items-center gap-2">
                <Check className="h-4 w-4 text-green-500" />
                <span>{t("logs.copied")}</span>
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <Clipboard className="h-4 w-4" />
                <span>{t("logs.print")}</span>
              </div>
            )}
          </Button>
          <DialogClose asChild>
            <Button variant="default">{t("close.button")}</Button>
          </DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
