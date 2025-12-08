import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useGameStore } from "@/lib/store";
import { useI18n } from "@/lib/i18n";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./ui/tooltip";
import { cn } from "@/lib/utils";

type Status = "idle" | "injecting" | "success" | "error";

type InjectionStateSnapshot = {
  status: Status;
  last_error: string | null;
  updated_at_ms: number;
};

export function InjectionStatusDot({ bordered = false }: { bordered?: boolean }) {
  const {
    injectionStatus,
    setInjectionStatus,
    lastInjectionError,
    setLastInjectionError,
  } = useGameStore();
  const { t } = useI18n();
  const toastShownRef = useRef<{ success?: boolean; errorMessage?: string }>({});
  const errorTimeoutRef = useRef<number | null>(null);
  const lastCleanupAtRef = useRef<number>(0);
  const lastSuccessAtRef = useRef<number>(0);
  const lastErrorRef = useRef<string | null>(null);

  const persistSnapshot = (status: Status, error: string | null) => {
    if (typeof window === "undefined") return;
    localStorage.setItem("injection:status", status);
    if (error) localStorage.setItem("injection:lastError", error);
    else localStorage.removeItem("injection:lastError");
  };

  const hydrateFromCache = () => {
    if (typeof window === "undefined") return;
    const cachedStatus = localStorage.getItem("injection:status") as Status | null;
    const cachedError = localStorage.getItem("injection:lastError");
    if (cachedStatus) {
      setInjectionStatus(cachedStatus);
      setLastInjectionError(cachedError ?? null);
    }
  };

  const showErrorToast = (message: string) => {
    if (!message) return;
    if (toastShownRef.current.errorMessage === message) return;
    toastShownRef.current.errorMessage = message;
    toast.error(message);
  };

  useEffect(() => {
    lastErrorRef.current = lastInjectionError;
  }, [lastInjectionError]);

  const showSuccessToast = () => {
    if (toastShownRef.current.success) return;
    toastShownRef.current.success = true;
    toastShownRef.current.errorMessage = undefined;
    toast.success(t("injection.success"));
  };

  useEffect(() => {
    let cancelled = false;

    const hydrate = async () => {
      try {
        const snapshot = await invoke<InjectionStateSnapshot>("get_injection_state");
        if (cancelled) return;
        const status = snapshot?.status ?? "idle";
        const lastError = snapshot?.last_error ?? null;
        setInjectionStatus(status);
        setLastInjectionError(lastError);
        persistSnapshot(status, lastError);
        if (status === "error" && lastError) {
          showErrorToast(lastError);
        }
      } catch (err) {
        console.warn("[InjectionStatus] hydrate failed, falling back to cache", err);
        hydrateFromCache();
      }
    };

    void hydrate();

    return () => {
      cancelled = true;
    };
  }, [setInjectionStatus, setLastInjectionError]);

  useEffect(() => {
    let unlistenStatus: () => void = () => { };
    let unlistenError: () => void = () => { };
    let unlistenTerminalLog: () => void = () => { };

    void (async () => {
      // Listen for terminal logs to detect cleanup messages emitted by the backend
      unlistenTerminalLog = await listen<string>("terminal-log", (e) => {
        const logMsg = e.payload;
        const current = useGameStore.getState().injectionStatus;
        if (
          (logMsg.includes("Cleaning up") && current === "success") ||
          logMsg.includes("Stopping skin injection process") ||
          logMsg.includes("Skin injection stopped")
        ) {
          lastCleanupAtRef.current = Date.now();
          setInjectionStatus("idle");
          setLastInjectionError(null);
          persistSnapshot("idle", null);
          toastShownRef.current = {};
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
            errorTimeoutRef.current = null;
          }
        }
      });

      // Accept both string and boolean payloads because some backend paths emit booleans
      unlistenStatus = await listen<unknown>("injection-status", (e) => {
        const payload = e.payload;
        const now = Date.now();

        const handleStatus = (status: Status) => {
          if (status === "injecting") {
            if (errorTimeoutRef.current) {
              clearTimeout(errorTimeoutRef.current);
              errorTimeoutRef.current = null;
            }
            setInjectionStatus("injecting");
            setLastInjectionError(null);
            persistSnapshot("injecting", null);
            toastShownRef.current.success = false;
          } else if (status === "success") {
            if (errorTimeoutRef.current) {
              clearTimeout(errorTimeoutRef.current);
              errorTimeoutRef.current = null;
            }
            lastSuccessAtRef.current = now;
            if (now >= lastCleanupAtRef.current) {
              setInjectionStatus("success");
              setLastInjectionError(null);
              persistSnapshot("success", null);
              toast.dismiss();
              showSuccessToast();
            }
          } else if (status === "error") {
            setInjectionStatus("error");
            persistSnapshot("error", lastErrorRef.current ?? null);
            if (errorTimeoutRef.current) {
              clearTimeout(errorTimeoutRef.current);
            }
            errorTimeoutRef.current = window.setTimeout(() => {
              setInjectionStatus("idle");
              persistSnapshot("idle", null);
              toastShownRef.current.success = false;
              errorTimeoutRef.current = null;
            }, 10000);
            showErrorToast(lastErrorRef.current ?? t("injection.error"));
          } else if (status === "idle") {
            setInjectionStatus("idle");
            persistSnapshot("idle", null);
            if (errorTimeoutRef.current) {
              clearTimeout(errorTimeoutRef.current);
              errorTimeoutRef.current = null;
            }
          }
        };

        if (typeof payload === "boolean") {
          handleStatus(payload ? "success" : "error");
          return;
        }

        const rawStatus = typeof payload === "string" ? payload : String(payload);
        const normalizedStatus: Status =
          rawStatus === "completed"
            ? "success"
            : rawStatus === "injecting" || rawStatus === "success" || rawStatus === "error" || rawStatus === "idle"
              ? (rawStatus as Status)
              : "error";

        handleStatus(normalizedStatus);
      });

      unlistenError = await listen<string>("skin-injection-error", (e) => {
        const message = e.payload;
        setInjectionStatus("error");
        setLastInjectionError(message);
        persistSnapshot("error", message);
        if (errorTimeoutRef.current) {
          clearTimeout(errorTimeoutRef.current);
        }
        errorTimeoutRef.current = window.setTimeout(() => {
          setInjectionStatus("idle");
          persistSnapshot("idle", null);
          toastShownRef.current.success = false;
          errorTimeoutRef.current = null;
        }, 10000);
        showErrorToast(message || t("injection.error"));
      });

      const devHandler = (e: Event) => {
        const ce = e as CustomEvent<unknown>;
        const payload = ce.detail;
        if (typeof payload === "boolean") {
          setInjectionStatus(payload ? "success" : "error");
          persistSnapshot(payload ? "success" : "error", null);
          if (!payload) showErrorToast(t("injection.error"));
          return;
        }

        const status = (typeof payload === "string" ? payload : String(payload)) as Status;
        setInjectionStatus(status);
        persistSnapshot(status, null);
        if (status === "error") showErrorToast(t("injection.error"));
      };

      window.addEventListener(
        "dev-injection-status",
        devHandler as EventListener
      );
    })();

    return () => {
      unlistenStatus();
      unlistenError();
      unlistenTerminalLog();
      if (errorTimeoutRef.current) {
        clearTimeout(errorTimeoutRef.current);
        errorTimeoutRef.current = null;
      }
    };
  }, [setInjectionStatus, setLastInjectionError, t]);

  const statusMeta: Record<Status, { color: string; animate?: string; label: string; shadow?: string }> = {
    injecting: {
      color: "bg-yellow-400",
      animate: "animate-pulse",
      label: t("injection.injecting"),
      shadow: "0 0 10px rgba(250,204,21,0.65)",
    },
    success: {
      color: "bg-green-500",
      label: t("injection.success"),
      shadow: "0 0 10px rgba(34,197,94,0.45)",
    },
    error: {
      color: "bg-red-500",
      label: t("injection.error"),
      shadow: "0 0 10px rgba(239,68,68,0.6)",
    },
    idle: {
      color: "bg-gray-400",
      label: t("injection.nothing"),
    },
  };

  const { color, animate, label, shadow } = statusMeta[injectionStatus];

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <div
            className={cn(
              "inline-flex items-center justify-center rounded-full",
              bordered ? "border px-2 py-1 bg-background/80" : ""
            )}
            aria-label={`Injection status: ${label}`}
          >
            <div
              className={cn(
                "h-3 w-3 rounded-full border border-border transition-all",
                color,
                animate,
              )}
              style={shadow ? { boxShadow: shadow } : undefined}
            />
          </div>
        </TooltipTrigger>
        <TooltipContent>{label}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
