"use client";

import { HelpCircle } from "lucide-react";
import { useGameStore } from "@/lib/store";
import { DropdownMenuItem } from "../ui/dropdown-menu";

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
    <DropdownMenuItem onClick={handleShowOnboarding}>
      <HelpCircle className="h-5 w-5" />
      Help
    </DropdownMenuItem>
  );
}
