"use client";

import { useTheme } from "next-themes";
import { Terminal, HelpCircle, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useState, useEffect } from "react";
import { 
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { HelpButton } from "@/components/onboarding/HelpButton";
import { TerminalLogsDialog } from "@/components/TerminalLogsDialog";

type Theme = {
  name: string;
  value: string;
};

export function MenuBar() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);
  const [availableThemes, setAvailableThemes] = useState<Theme[]>([
    { name: "Default", value: "default" },
    { name: "Red", value: "red" },
    { name: "Rose", value: "rose" },
    { name: "Orange", value: "orange" },
    { name: "Green", value: "green" }, 
    { name: "Blue", value: "blue" },
    { name: "Yellow", value: "yellow" },
    { name: "Violet", value: "violet" },
  ]);

  // When mounted on client, now we can show the UI
  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return null;
  }

  return (
    <div className="fixed top-0 right-0 flex items-center gap-2 p-4 z-50">
      <TerminalLogsDialog />
      
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="outline" size="icon" className="rounded-full">
            <span className="sr-only">Toggle theme</span>
            <div 
              className="w-4 h-4 rounded-full" 
              style={{ 
                backgroundColor: `var(--primary)`
              }} 
            />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuLabel>Theme Color</DropdownMenuLabel>
          <DropdownMenuSeparator />
          {availableThemes.map((t) => (
            <DropdownMenuItem
              key={t.value}
              onClick={() => setTheme(t.value.toLowerCase())}
              className="flex items-center justify-between cursor-pointer"
            >
              <span>{t.name}</span>
              {theme?.toLowerCase() === t.value.toLowerCase() && (
                <Check className="h-4 w-4" />
              )}
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuLabel>Mode</DropdownMenuLabel>
          <DropdownMenuItem onClick={() => setTheme("light")}>
            <span>Light</span>
            {theme === "light" && <Check className="h-4 w-4 ml-auto" />}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setTheme("dark")}>
            <span>Dark</span>
            {theme === "dark" && <Check className="h-4 w-4 ml-auto" />}
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setTheme("system")}>
            <span>System</span>
            {theme === "system" && <Check className="h-4 w-4 ml-auto" />}
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      
      <HelpButton />
    </div>
  );
}