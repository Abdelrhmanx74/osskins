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
import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Terminal, Trash2, X } from "lucide-react";

export function TerminalLogsDialog() {
  const [logs, setLogs] = useState<string[]>([]);
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

  useEffect(() => {
    // Scroll to bottom when logs change
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const clearLogs = () => {
    setLogs([]);
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline" size="icon">
          <Terminal className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-5xl">
        <DialogHeader className="flex flex-row items-center justify-between">
          <DialogTitle>Terminal Logs</DialogTitle>
          <div className="flex flex-row items-center gap-2">
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
        <ScrollArea className="h-[500px] w-full rounded-md border p-4">
          <div className="font-mono text-sm">
            {logs.map((log, index) => (
              <div key={index} className="whitespace-pre-wrap">
                {log}
              </div>
            ))}
            <div ref={bottomRef} />
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
