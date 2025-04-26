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
  const { injectionStatus, setInjectionStatus } = useGameStore();
  const toastShownRef = useRef<Record<string, boolean>>({});

  // Listen for injection start/stop and error events
  useEffect(() => {
    // Setup listeners and store unlisten functions
    let unlistenStatus: () => void = () => {};
    let unlistenError: () => void = () => {};

    void (async () => {
      unlistenStatus = await listen("injection-status", (e) => {
        const status = e.payload;

        if (status === "injecting") {
          setInjectionStatus("injecting");
          // Reset toast tracking when starting new injection
          toastShownRef.current = {};
        } else if (status === "success") {
          setInjectionStatus("success");
          // Only show success toast if we haven't shown one for this injection cycle
          if (!toastShownRef.current.success) {
            toast.success("Skin injection completed successfully");
            toastShownRef.current.success = true;
          }
        } else if (status === "error") {
          setInjectionStatus("error");
          // Error message handled by separate event
        } else {
          // Default to idle for any other status
          setInjectionStatus("idle");
        }
      });

      unlistenError = await listen<string>("skin-injection-error", (e) => {
        setInjectionStatus("error");
        // Only show error toast if we haven't shown one for this error
        if (!toastShownRef.current.error) {
          toast.error(`Skin injection failed: ${e.payload}`);
          toastShownRef.current.error = true;
        }
      });
    })();

    return () => {
      unlistenStatus();
      unlistenError();
    };
  }, [setInjectionStatus]);

  // Auto-reset back to idle after showing success/error
  useEffect(() => {
    if (injectionStatus === "success" || injectionStatus === "error") {
      const t = setTimeout(() => {
        setInjectionStatus("idle");
        // Clear toast tracking when returning to idle
        toastShownRef.current = {};
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
          className={`h-3 w-3 rounded-full border border-border ${color} ${animate}`}
          aria-label={label}
        />
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
