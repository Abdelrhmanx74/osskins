import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { useGameStore } from "@/lib/store";
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export function GameStatusDot() {
  const { lcuStatus } = useGameStore();
  const [status, setStatus] = useState<string>("None");

  useEffect(() => {
    // Listen for LCU status events
    const unlistenStatus = listen("lcu-status", (event) => {
      setStatus(event.payload as string);
    });

    // Cleanup listener
    return () => {
      void unlistenStatus.then((fn) => {
        fn();
      });
    };
  }, []);

  let color = "bg-red-500"; // Disconnected
  let animate = "";
  let label = "Disconnected";

  switch (status) {
    case "ChampSelect":
      color = "bg-yellow-400";
      label = "Champion Select";
      break;
    case "InProgress":
      color = "bg-green-500";
      label = "In Game";
      animate = "animate-pulse";
      break;
    case "Reconnect":
      color = "bg-yellow-400";
      label = "Reconnecting";
      animate = "animate-pulse";
      break;
    case "None":
      color = "bg-gray-500";
      label = "Idle";
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
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
