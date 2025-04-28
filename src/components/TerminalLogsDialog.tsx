import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Terminal, Trash2, X, Copy, ArrowDownToLine } from "lucide-react";
import { DropdownMenuItem } from "./ui/dropdown-menu";
import { toast } from "sonner";
import clsx from "clsx";

export function TerminalLogsDialog() {
  const [logs, setLogs] = useState<string[]>([]);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const scrollAreaRef = useRef<HTMLDivElement | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    // Listen for terminal logs from the backend
    const unlisten = listen("terminal-log", (event) => {
      const log = event.payload as string;
      setLogs((prev) => [...prev, log]);
    });

    return () => {
      void unlisten.then((fn) => {
        fn();
      });
    };
  }, []);

  // Scroll to bottom only if following
  useEffect(() => {
    if (isAtBottom) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, isAtBottom]);

  // Detect if user is at bottom
  const handleScroll = useCallback(() => {
    const el = scrollAreaRef.current;
    if (!el) return;
    // 20px threshold for "at bottom"
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 20;
    setIsAtBottom(atBottom);
  }, []);

  const clearLogs = () => {
    setLogs([]);
  };

  const copyLogs = async () => {
    try {
      await navigator.clipboard.writeText(logs.join("\n"));
      toast.success("Logs copied to clipboard");
    } catch {
      toast.error("Failed to copy logs");
    }
  };

  // Scroll to bottom and enable follow
  const scrollToBottom = () => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    setIsAtBottom(true);
  };

  return (
    <Dialog modal>
      <DialogTrigger asChild>
        <DropdownMenuItem
          onSelect={(e) => {
            e.preventDefault();
          }}
        >
          <Terminal className="h-4 w-4" />
          Terminal Logs
        </DropdownMenuItem>
      </DialogTrigger>
      <DialogContent className="sm:max-w-5xl">
        <DialogHeader className="flex flex-row items-center justify-between">
          <DialogTitle>Terminal Logs</DialogTitle>
          <div className="flex flex-row items-center gap-2">
            <Button
              title="Copy"
              variant="outline"
              size="icon"
              onClick={() => {
                void copyLogs();
              }}
              disabled={logs.length === 0}
            >
              <Copy className="h-4 w-4" />
            </Button>
            <Button
              title="Clear"
              variant="outline"
              size="icon"
              onClick={clearLogs}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
            <DialogClose title="Close" asChild>
              <Button variant="outline" size="icon">
                <X className="h-4 w-4" />
              </Button>
            </DialogClose>
          </div>
        </DialogHeader>
        <div className="relative">
          <ScrollArea
            className="h-[500px] w-full max-w- rounded-md border p-4"
            ref={scrollAreaRef}
            onScroll={handleScroll}
            style={{ overflowY: "auto", position: "relative" }}
          >
            <div className="font-mono text-sm">
              {logs.map((log, index) => (
                <div key={index} className="whitespace-pre-wrap">
                  {log}
                </div>
              ))}
              <div ref={bottomRef} />
            </div>
          </ScrollArea>
          {!isAtBottom && (
            <Button
              onClick={scrollToBottom}
              variant="secondary"
              size="icon"
              className={clsx(
                "absolute right-4 bottom-6 z-10 shadow-lg animate-in fade-in",
                "bg-background/80 backdrop-blur"
              )}
              title="Scroll to bottom"
            >
              <ArrowDownToLine className="h-5 w-5" />
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
