"use client";
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTerminalLogStore, TerminalLog } from "@/lib/store";

export function TerminalLogListener() {
  useEffect(() => {
    const unlisten = listen("terminal-log", (event) => {
      const log = event.payload as TerminalLog;
      if (
        log &&
        typeof log === "object" &&
        "message" in log &&
        "log_type" in log &&
        "timestamp" in log
      ) {
        useTerminalLogStore.getState().addLog(log);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);
  return null;
}
