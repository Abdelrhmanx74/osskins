"use client";

import { useState } from "react";
import { HelpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useGameStore } from "@/lib/store";

export function HelpButton() {
  const { setHasCompletedOnboarding } = useGameStore();

  const handleShowOnboarding = () => {
    // Reset the onboarding status
    localStorage.removeItem("onboardingShown");
    setHasCompletedOnboarding(false);

    // Force reload to trigger onboarding
    window.location.reload();
  };

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          onClick={handleShowOnboarding}
          className="text-muted-foreground hover:text-foreground rounded-full"
        >
          <HelpCircle className="h-5 w-5" />
        </Button>
      </TooltipTrigger>
      <TooltipContent>Show onboarding guide</TooltipContent>
    </Tooltip>
  );
}
