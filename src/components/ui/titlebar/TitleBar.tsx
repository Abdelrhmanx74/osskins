"use client";

import { useState, useEffect, MouseEvent } from "react";
import {
  WebviewWindow,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { Button } from "@/components/ui/button";
import { Minus, Square, X } from "lucide-react";

interface TitleBarProps {
  title?: string;
}

export function TitleBar({ title = "League Skin Manager" }: TitleBarProps) {
  const [isMaximized, setIsMaximized] = useState(false);
  const [appWindow, setAppWindow] = useState<WebviewWindow | null>(null);

  useEffect(() => {
    // Get the current webview window
    const initWindow = async () => {
      try {
        const currentWindow = getCurrentWebviewWindow();
        setAppWindow(currentWindow);

        // Check if the window is maximized initially
        try {
          const maximized = await currentWindow.isMaximized();
          setIsMaximized(maximized);
        } catch (error) {
          console.error("Failed to check if window is maximized:", error);
        }

        // Listen for window resize events
        const unlistenResize = await currentWindow.listen(
          "tauri://resize",
          () => {
            currentWindow
              .isMaximized()
              .then((maximized) => {
                setIsMaximized(maximized);
              })
              .catch((error: unknown) => {
                console.error(
                  "Failed to check if window is maximized on resize:",
                  error
                );
              });
          }
        );

        return unlistenResize;
      } catch (error) {
        console.error("Failed to initialize window:", error);
        return null;
      }
    };

    let unlisten: (() => void) | null = null;
    initWindow()
      .then((unlistenFn) => {
        unlisten = unlistenFn;
      })
      .catch((error: unknown) => {
        console.error("Failed to initialize window listeners:", error);
      });

    return () => {
      // Cleanup event listeners
      if (unlisten) unlisten();
    };
  }, []);

  const handleDragStart = async (e: MouseEvent) => {
    try {
      if (appWindow) {
        // Make sure we only handle dragging from the titlebar area
        if ((e.target as HTMLElement).closest("[data-tauri-drag-region]")) {
          await appWindow.startDragging();
        }
      }
    } catch (error) {
      console.error("Failed to start dragging:", error);
    }
  };

  const minimize = async () => {
    try {
      if (appWindow) {
        await appWindow.minimize();
      }
    } catch (error) {
      console.error("Failed to minimize window:", error);
    }
  };

  const toggleMaximize = async () => {
    try {
      if (appWindow) {
        if (isMaximized) {
          await appWindow.unmaximize();
        } else {
          await appWindow.maximize();
        }
      }
    } catch (error) {
      console.error("Failed to toggle maximize:", error);
    }
  };

  const close = async () => {
    try {
      if (appWindow) {
        await appWindow.close();
      }
    } catch (error) {
      console.error("Failed to close window:", error);
    }
  };

  return (
    <div className="flex items-center">
      <Button
        variant="ghost"
        size="icon"
        className="p-4"
        onClick={() => void minimize()}
        aria-label="Minimize"
      >
        <Minus className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="p-4"
        onClick={() => void toggleMaximize()}
        aria-label={isMaximized ? "Restore" : "Maximize"}
      >
        <Square className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="p-4 dark:hover:bg-destructive hover:text-destructive-foreground"
        onClick={() => void close()}
        aria-label="Close"
      >
        <X className="h-4 w-4" />
      </Button>
    </div>
  );
}
