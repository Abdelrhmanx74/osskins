"use client";

import { DropdownMenuItem } from "@/components/ui/dropdown-menu";
import { Terminal } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export function PrintLogsButton() {
  const handle = async () => {
    try {
      const path = await invoke<string>("print_logs");
      toast.success(`Logs saved to ${path}`);
    } catch (e) {
      console.error(e);
      toast.error("Failed to print logs");
    }
  };

  return (
    <DropdownMenuItem
      onClick={() => {
        void handle();
        return undefined;
      }}
      onSelect={(e) => {
        e.preventDefault();
        return undefined;
      }}
    >
      <Terminal className="h-4 w-4" />
      Print logs
    </DropdownMenuItem>
  );
}
