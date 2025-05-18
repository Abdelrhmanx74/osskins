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
import {
  Terminal,
  Trash2,
  X,
  Copy,
  ArrowDownToLine,
  Filter,
} from "lucide-react";
import {
  DropdownMenuItem,
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
} from "@/components/ui/dropdown-menu";
import { toast } from "sonner";
import clsx from "clsx";
import { useTerminalLogStore, TerminalLog } from "@/lib/store";
import { Switch } from "./ui/switch";

// Log type options for filtering
const LOG_TYPE_LABELS: Record<string, string> = {
  all: "All",
  "lcu-watcher": "LCU Watcher",
  injection: "Injection",
  error: "Error",
  debug: "Debug",
};
const ALL_LOG_TYPES = Object.keys(LOG_TYPE_LABELS).filter((k) => k !== "all");

export function TerminalLogsDialog() {
  const logs = useTerminalLogStore((s) => s.logs);
  const clearLogs = useTerminalLogStore((s) => s.clearLogs);
  const [filter, setFilter] = useState<string[]>(ALL_LOG_TYPES);
  const [autoScroll, setAutoScroll] = useState(true);
  const [isAtBottom, setIsAtBottom] = useState(true);
  const scrollAreaRef = useRef<HTMLDivElement | null>(null);
  const bottomRef = useRef<HTMLDivElement | null>(null);

  // Remove the duplicate event listener - we now use the global TerminalLogListener

  // Scroll to bottom only if following
  useEffect(() => {
    if (autoScroll && isAtBottom) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, autoScroll, isAtBottom]);

  // Detect if user is at bottom
  const handleScroll = useCallback(() => {
    const el = scrollAreaRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 20;
    setIsAtBottom(atBottom);
  }, []);

  const copyLogs = async () => {
    try {
      await navigator.clipboard.writeText(
        logs
          .filter((log) => filter.length === 0 || filter.includes(log.log_type))
          .map((log) => `[${log.log_type}] ${log.message}`)
          .join("\n")
      );
      toast.success("Logs copied to clipboard");
    } catch {
      toast.error("Failed to copy logs");
    }
  };

  const scrollToBottom = () => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    setAutoScroll(true);
    setIsAtBottom(true);
  };

  // Multi-select filter logic
  const toggleFilter = (type: string) => {
    setFilter((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    );
  };

  const filteredLogs = logs.filter(
    (log) => filter.length === 0 || filter.includes(log.log_type)
  );

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
      <DialogContent className="sm:max-w-5xl overflow-y-auto">
        <DialogHeader className="flex flex-row items-center justify-between">
          <DialogTitle>Terminal Logs</DialogTitle>
          <div className="flex flex-row items-center gap-2">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="icon" title="Filter logs">
                  <Filter className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {Object.entries(LOG_TYPE_LABELS)
                  .filter(([key]) => key !== "all")
                  .map(([key, label]) => (
                    <DropdownMenuCheckboxItem
                      key={key}
                      checked={filter.includes(key)}
                      onCheckedChange={() => toggleFilter(key)}
                    >
                      {label}
                    </DropdownMenuCheckboxItem>
                  ))}
              </DropdownMenuContent>
            </DropdownMenu>
            <Button
              title="Copy"
              variant="outline"
              size="icon"
              onClick={() => {
                void copyLogs();
              }}
              disabled={filteredLogs.length === 0}
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
        <ScrollArea
          className="size-full relative h-[500px] max-w-[950px] rounded-md border p-4 font-mono text-sm select-text"
          ref={scrollAreaRef}
          onScroll={handleScroll}
        >
          {filteredLogs.map((log, index) => (
            <div key={index} className="whitespace-pre-wrap">
              <span className="text-xs font-bold mr-2">[{log.log_type}]</span>
              {log.message}
            </div>
          ))}
          <div ref={bottomRef} />
        </ScrollArea>
        {!isAtBottom && autoScroll && (
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
        <div className="flex items-center gap-2">
          <span className="text-xs">Auto Scroll</span>
          <Switch
            checked={autoScroll}
            onCheckedChange={setAutoScroll}
            aria-label="Auto Scroll"
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}
