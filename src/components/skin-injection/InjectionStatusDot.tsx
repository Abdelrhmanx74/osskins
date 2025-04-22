import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { useGameStore } from "@/lib/store";
import type { InjectionStatus } from "@/lib/store"; // Import the type
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";

export function InjectionStatusDot() {
  // Use the new state and setter
  const { injectionStatus, setInjectionStatus, setLcuStatus } = useGameStore();
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    // Listen for injection status events (start/end)
    const unlistenStatus = listen("injection-status", (event) => {
      const injecting = event.payload as boolean;
      if (injecting) {
        setInjectionStatus("injecting");
        setErrorMessage(null); // Clear previous error/message on new injection start
        toast.info("Starting skin injection...");
      }
    });

    // Listen for injection success event
    const unlistenSuccess = listen("injection-success", () => {
      setInjectionStatus("success");
      setErrorMessage(null); // Clear error message on success
      toast.success("Skin injection completed successfully");
    });

    // Listen for injection error events
    const unlistenError = listen("skin-injection-error", (event) => {
      const errorMsg = event.payload as string;
      setErrorMessage(errorMsg);
      setInjectionStatus("error"); // Set status to error

      // Show error toast with more details
      toast.error("Skin Injection Failed", {
        description: errorMsg,
        duration: 5000,
      });
    });

    // Listen for LCU status changes to reset injection status
    const unlistenLcu = listen("lcu-status", (event) => {
      const status = event.payload as string;
      setLcuStatus(status); // Update LCU status in store

      // Show status change toast
      if (status === "ChampSelect") {
        toast.info("Champion Select detected, ready for skin injection");
      } else if (status === "InProgress") {
        toast.success("Game started, skins should be active");
      } else if (status === "EndOfGame") {
        toast.info("Game ended, cleaning up skin injection");
      }

      // Reset injection status if game ends or returns to a non-injectable state
      if (status === "None" || status === "Lobby" || status === "EndOfGame") {
        // Only reset if currently in a final state (success/error)
        const currentInjectionStatus = useGameStore.getState().injectionStatus;
        if (
          currentInjectionStatus === "success" ||
          currentInjectionStatus === "error"
        ) {
          setInjectionStatus("idle");
          setErrorMessage(null); // Clear error message when resetting to idle
        }
      }
    });

    // Cleanup listeners
    return () => {
      void unlistenStatus.then((fn) => {
        fn();
      });
      void unlistenSuccess.then((fn) => {
        fn();
      });
      void unlistenError.then((fn) => {
        fn();
      });
      void unlistenLcu.then((fn) => {
        fn();
      });
    };
  }, [setInjectionStatus, setLcuStatus]);

  let color = "bg-gray-500"; // Idle state
  let animate = "";
  let label = "Idle";

  // Determine appearance based on the injectionStatus state
  switch (injectionStatus) {
    case "injecting":
      color = "bg-yellow-400";
      animate = "animate-pulse";
      label = "Injecting...";
      break;
    case "success":
      color = "bg-green-500";
      label = "Injected";
      break;
    case "error":
      color = "bg-red-500";
      label = "Error";
      break;
    case "idle":
    default:
      // Keep default gray color and "Idle" label
      break;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <div
          className={`h-3 w-3 rounded-full border border-border ${color} ${animate}`}
          aria-label={label}
        />
      </TooltipTrigger>
      <TooltipContent>
        {/* Show error message specifically if in error state, otherwise show the status label */}
        {injectionStatus === "error" && errorMessage
          ? `Error: ${errorMessage}`
          : label}
      </TooltipContent>
    </Tooltip>
  );
}
