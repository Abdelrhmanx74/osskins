"use client";

import { useState, useEffect } from "react";
import {
  WebviewWindow,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Minus, EyeOff, X } from "lucide-react";

export function TitleBar() {
  const [appWindow, setAppWindow] = useState<WebviewWindow | null>(null);

  useEffect(() => {
    // Get the current webview window
    const initWindow = async () => {
      try {
        const currentWindow = getCurrentWebviewWindow();
        setAppWindow(currentWindow);
      } catch (error) {
        console.error("Failed to initialize window:", error);
      }
    };

    void initWindow();
  }, []);

  const minimize = async () => {
    try {
      if (appWindow) {
        await appWindow.minimize();
      }
    } catch (error) {
      console.error("Failed to minimize window:", error);
    }
  };

  const hideInTray = async () => {
    try {
      await invoke("hide_window");
    } catch (error) {
      console.error("Failed to invoke hide_window:", error);
    }
  };

  const close = async () => {
    try {
      await invoke("exit_app");
    } catch (error) {
      console.error("Failed to invoke exit_app:", error);
    }
  };

  return (
    <div className="flex items-center">
      <Button
        variant="ghost"
        size="icon"
        className="p-4"
        onClick={() => {
          void minimize();
          return undefined;
        }}
        aria-label="Minimize"
      >
        <Minus className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="p-4"
        onClick={() => {
          void hideInTray();
          return undefined;
        }}
        aria-label="Hide in tray"
      >
        <EyeOff className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="p-4 dark:hover:bg-destructive hover:text-destructive-foreground"
        onClick={() => {
          void close();
          return undefined;
        }}
        aria-label="Close"
      >
        <X className="h-4 w-4" />
      </Button>
    </div>
  );
}
