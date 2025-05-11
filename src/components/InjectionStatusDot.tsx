import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { useGameStore } from "@/lib/store";

type Status = "idle" | "injecting" | "success" | "error";

export function InjectionStatusDot() {
  const injectionStatus = useGameStore((s) => s.injectionStatus);
  const setInjectionStatus = useGameStore((s) => s.setInjectionStatus);
  const toastShownRef = useRef<Record<string, boolean>>({});
  const currentInjectionRef = useRef<string | null>(null);

  // Listen for injection start/stop and error events
  useEffect(() => {
    // Setup listeners and store unlisten functions
    let unlistenStatus: () => void = () => {};
    let unlistenError: () => void = () => {};
    let unlistenTerminalLog: () => void = () => {};

    void (async () => {
      // Track terminal logs for warnings during overlay creation
      unlistenTerminalLog = await listen<string>("terminal-log", (e) => {
        const logMsg = e.payload;

        // If we see a log that indicates we're retrying, mark this as a retry situation
        if (
          logMsg.includes("Retrying overlay creation") ||
          logMsg.includes("Access violation error")
        ) {
          currentInjectionRef.current = "retrying";
        }

        // Clear any existing error toasts when we see a success message
        if (
          logMsg.includes("Overlay creation succeeded") ||
          logMsg.includes("Overlay process started successfully") ||
          logMsg.includes("Skin injection completed successfully")
        ) {
          // Clear any error toasts since we recovered
          if (currentInjectionRef.current === "retrying") {
            // We've recovered from a retry, waiting for final success
            currentInjectionRef.current = "recovered";
          }
        }
      });

      unlistenStatus = await listen("injection-status", (e) => {
        const status = e.payload;

        if (status === "injecting") {
          setInjectionStatus("injecting");
          // Reset toast tracking when starting new injection
          toastShownRef.current = {};
          currentInjectionRef.current = "injecting";
        } else if (status === "completed" || status === "success") {
          setInjectionStatus("success");
          // Hide any pending error toasts
          toast.dismiss();

          // Show success toast
          toast.success("Skin injection completed successfully");
          toastShownRef.current.success = true;
          currentInjectionRef.current = null;
        } else if (status === "error") {
          // Only show error toast if we're not in a retry situation or if injection has actually failed
          if (
            currentInjectionRef.current !== "retrying" &&
            currentInjectionRef.current !== "recovered"
          ) {
            setInjectionStatus("error");
            // Error message handled by separate event
          }
        } else {
          // Default to idle for any other status
          setInjectionStatus("idle");
          currentInjectionRef.current = null;
        }
      });

      unlistenError = await listen<string>("skin-injection-error", (e) => {
        // Only show actual errors, not temporary failures during retries
        if (
          currentInjectionRef.current !== "retrying" &&
          currentInjectionRef.current !== "recovered"
        ) {
          setInjectionStatus("error");
          // Only show error toast if we haven't shown one for this error
          if (!toastShownRef.current.error) {
            toast.error(`Skin injection failed: ${e.payload}`);
            toastShownRef.current.error = true;
          }
        }
      });
    })();

    return () => {
      unlistenStatus();
      unlistenError();
      unlistenTerminalLog();
    };
  }, [setInjectionStatus]);

  // Auto-reset back to idle after showing success/error
  useEffect(() => {
    if (injectionStatus === "success" || injectionStatus === "error") {
      const t = setTimeout(() => {
        setInjectionStatus("idle");
        // Clear toast tracking when returning to idle
        toastShownRef.current = {};
        currentInjectionRef.current = null;
      }, 5000); // Extended time to 5 seconds to make status more visible

      return () => {
        clearTimeout(t);
      };
    }
  }, [injectionStatus, setInjectionStatus]);

  // Map status to color, label, animation
  let color = "";
  let animate = "";
  let label = "";

  switch (injectionStatus) {
    case "injecting":
      color = "bg-yellow-400";
      animate = "animate-pulse";
      label = "Injecting skins...";
      break;
    case "success":
      color = "bg-green-500";
      label = "Injection successful";
      break;
    case "error":
      color = "bg-red-500";
      label = "Injection failed";
      break;
    default:
      color = "bg-gray-500";
      label = "Ready";
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          className={`size-3 aspect-square shrink-0 rounded-full border border-border ${color} ${animate}`}
          aria-label={label}
        />
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
