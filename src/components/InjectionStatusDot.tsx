import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { useGameStore } from "@/lib/store";
import { Separator } from "./ui/separator";

type Status = "idle" | "injecting" | "success" | "error";

export function InjectionStatusDot({
  showLabel = false,
  bordered = false,
}: {
  showLabel?: boolean;
  bordered?: boolean;
}) {
  const { injectionStatus, setInjectionStatus } = useGameStore();
  const toastShownRef = useRef<Record<string, boolean>>({});
  const errorTimeoutRef = useRef<number | null>(null);
  const lastCleanupAtRef = useRef<number>(0);
  const lastSuccessAtRef = useRef<number>(0);

  // Behavior desired:
  // - Yellow pulsing while injection process is running ("injecting")
  // - Green when files are injected / overlay running (we treat "success" as injected)
  // - Gray (or red label) when nothing is injected ("idle")
  // Keep green until a cleanup is detected. The backend logs contain "Cleaning up" messages
  // and also emits "injection-status" events. We'll listen for both.

  useEffect(() => {
    let unlistenStatus: () => void = () => {};
    let unlistenError: () => void = () => {};
    let unlistenTerminalLog: () => void = () => {};

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

        // If backend emitted a boolean (some code paths do), map true->success, false->error
        if (typeof payload === "boolean") {
          // clear any pending error auto-reset when new status arrives
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
            errorTimeoutRef.current = null;
          }
          if (payload) {
            lastSuccessAtRef.current = now;
            // only accept success if it is not older than the last cleanup
            if (now >= lastCleanupAtRef.current) {
              setInjectionStatus("success");
              toast.dismiss();
              if (!toastShownRef.current.success) {
                toast.success("Skin injection completed successfully");
                toastShownRef.current.success = true;
              }
            } else {
              // stale success event; ignore
              return;
            }
          } else {
            // boolean false indicates injection failure in some backend paths
            setInjectionStatus("error");
            if (errorTimeoutRef.current) {
              clearTimeout(errorTimeoutRef.current);
            }
            errorTimeoutRef.current = window.setTimeout(() => {
              setInjectionStatus("idle");
              toastShownRef.current = {};
              errorTimeoutRef.current = null;
            }, 10000);
            if (!toastShownRef.current.error) {
              toast.error("Skin injection failed");
              toastShownRef.current.error = true;
            }
          }
          return;
        }

        const status = typeof payload === "string" ? payload : String(payload);
        if (status === "injecting") {
          // clear any pending error auto-reset when injection restarts
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
            errorTimeoutRef.current = null;
          }
          setInjectionStatus("injecting");
          toastShownRef.current = {};
        } else if (status === "completed" || status === "success") {
          // Completed means injection finished and files/overlay should be active -> keep green until cleanup
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
            errorTimeoutRef.current = null;
          }
          const now = Date.now();
          lastSuccessAtRef.current = now;
          if (now >= lastCleanupAtRef.current) {
            setInjectionStatus("success");
            // dismiss any pending error toasts and show success once
            toast.dismiss();
            if (!toastShownRef.current.success) {
              toast.success("Skin injection completed successfully");
              toastShownRef.current.success = true;
            }
          } else {
            // stale success; ignore
            return;
          }
        } else if (status === "error") {
          // show error (red) and schedule revert to idle in case backend doesn't emit cleanup
          setInjectionStatus("error");
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
          }
          errorTimeoutRef.current = window.setTimeout(() => {
            setInjectionStatus("idle");
            toastShownRef.current = {};
            errorTimeoutRef.current = null;
          }, 10000);
        } else if (status === "idle") {
          // Backend explicitly reported idle/cleaned
          setInjectionStatus("idle");
          if (errorTimeoutRef.current) {
            clearTimeout(errorTimeoutRef.current);
            errorTimeoutRef.current = null;
          }
        }
      });

      unlistenError = await listen<string>("skin-injection-error", (e) => {
        // transient error state: show red, then go back to idle after timeout (or earlier if cleanup runs)
        setInjectionStatus("error");
        if (errorTimeoutRef.current) {
          clearTimeout(errorTimeoutRef.current);
        }
        errorTimeoutRef.current = window.setTimeout(() => {
          setInjectionStatus("idle");
          toastShownRef.current = {};
          errorTimeoutRef.current = null;
        }, 10000);
        if (!toastShownRef.current.error) {
          toast.error(`Skin injection failed: ${e.payload}`);
          toastShownRef.current.error = true;
        }
      });

      // Development helper: listen to a DOM CustomEvent so you can simulate events
      // from the browser console without using Tauri. Dispatch like:
      // window.dispatchEvent(new CustomEvent('dev-injection-status', { detail: 'injecting' }))
      const devHandler = (e: Event) => {
        const ce = e as CustomEvent<unknown>;
        const payload = ce.detail;

        // reuse same mapping logic as the Tauri 'injection-status' listener
        if (typeof payload === "boolean") {
          if (payload) {
            setInjectionStatus("success");
            toast.dismiss();
          } else {
            setInjectionStatus("error");
          }
          return;
        }

        const status = typeof payload === "string" ? payload : String(payload);
        if (status === "injecting") setInjectionStatus("injecting");
        else if (status === "completed" || status === "success")
          setInjectionStatus("success");
        else if (status === "error") setInjectionStatus("error");
        else if (status === "idle") setInjectionStatus("idle");
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
  }, [setInjectionStatus]);

  // Map status to color, label, animation. Keep green after success until cleanup sets idle.
  let color = "";
  let animate = "";
  let label = "";

  let inlineStyle: React.CSSProperties | undefined;
  switch (injectionStatus) {
    case "injecting":
      color = "bg-yellow-400";
      // whole-dot glow: persistent pulse; use inline boxShadow to guarantee it's present in all builds
      animate = "animate-pulse";
      inlineStyle = { boxShadow: "0 0 12px rgba(250,204,21,0.6)" };
      label = "Injecting skins...";
      break;
    case "success":
      color = "bg-green-500";
      label = "Injected - overlay running";
      break;
    case "error":
      color = "bg-red-500";
      label = "Injection error";
      break;
    default:
      color = "bg-gray-500";
      label = "Nothing injected";
  }

  return (
    <div
      className={`px-2 py-1 rounded-full flex items-center gap-2 ${color} ${animate}`}
      aria-label={`Injection status: ${label}`}
    >
      {/* <Separator className="mr-3" orientation="vertical" /> */}
      {/* <div
        className={`size-3 aspect-square shrink-0 rounded-full border border-border ${color} ${animate}`}
        style={inlineStyle}
        aria-hidden
      /> */}
      {showLabel && (
        <div className="text-sm font-medium leading-none whitespace-nowrap">
          {label}
        </div>
      )}
      {/* <Separator className="ml-3" orientation="vertical" /> */}
    </div>
  );
}
