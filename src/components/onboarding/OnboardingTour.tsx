"use client";

import React, { useState, useEffect } from "react";
import {
  X,
  ArrowRight,
  Check,
  Search,
  Heart,
  Play,
  RefreshCw,
  Info,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useGameStore } from "@/lib/store";
import "./onboarding.css";

const steps = [
  {
    title: "Welcome to League Skin Manager!",
    description: "This guide will show you how to use the main features.",
    target: null,
    icon: <Info className="h-5 w-5" />,
  },
  {
    title: "Search for Champions",
    description: "Use the search bar to find champions by name.",
    target: ".relative.flex-1", // The search input container
    highlight: true,
    icon: <Search className="h-5 w-5" />,
  },
  {
    title: "Favorite Champions",
    description: "Click the heart icon to add a champion to your favorites.",
    target: ".champion-card",
    highlight: true,
    icon: <Heart className="h-5 w-5" />,
  },
  {
    title: "Select a Champion",
    description: "Click a champion card to view their available skins.",
    target: ".champion-card",
    highlight: true,
    icon: <Play className="h-5 w-5" />,
  },
  {
    title: "Choose a Skin",
    description:
      "Click a skin to select it. The selected skin will be used in your next game.",
    target: null,
    icon: <Play className="h-5 w-5" />,
  },
  {
    title: "Game Integration",
    description:
      "When you start a game, your selected skin will be applied automatically. The status dot shows connection status.",
    target: ".h-3.w-3.rounded-full", // Target the status dot
    highlight: true,
  },
  {
    title: "Update Data",
    description:
      "Click 'Update Data' after a new patch to refresh champion and skin info in case of a new patch.",
    target: "button:has(.h-4.w-4)", // Button with RefreshCw icon
    highlight: true,
    icon: <RefreshCw className="h-5 w-5" />,
  },
  {
    title: "Done",
    description:
      "You can now use the app. Open this guide again anytime from the help button.",
    target: null,
    icon: <Check className="h-5 w-5" />,
  },
];

export function OnboardingTour() {
  const [isOpen, setIsOpen] = useState(false);
  const [currentStep, setCurrentStep] = useState(0);
  const [hasSeenOnboarding, setHasSeenOnboarding] = useState(false);
  const { setHasCompletedOnboarding } = useGameStore();
  const [highlightedElement, setHighlightedElement] = useState<Element | null>(
    null
  );

  useEffect(() => {
    // Check if the user has seen the onboarding before
    const onboardingShown = localStorage.getItem("onboardingShown");
    if (!onboardingShown) {
      setIsOpen(true);
    } else {
      setHasSeenOnboarding(true);
    }
  }, []);

  // Handle highlighting elements based on current step
  useEffect(() => {
    if (!isOpen) return;

    const step = steps[currentStep];

    // Clear previous highlight
    if (highlightedElement) {
      highlightedElement.classList.remove(
        "onboarding-highlight",
        "target-highlight"
      );
      setHighlightedElement(null);
    }

    // Apply new highlight if target exists
    if (step.target) {
      setTimeout(() => {
        const element = document.querySelector(step.target);
        if (element) {
          element.scrollIntoView({ behavior: "smooth", block: "center" });

          if (step.highlight) {
            element.classList.add("target-highlight");
            if (currentStep > 0) {
              element.classList.add("onboarding-highlight");
            }
            setHighlightedElement(element);
          }
        }
      }, 300);
    }

    // Add onboarding-active class to body when onboarding is open
    document.body.classList.toggle("onboarding-active", isOpen);

    return () => {
      // Cleanup function
      document.body.classList.remove("onboarding-active");
      if (highlightedElement) {
        highlightedElement.classList.remove(
          "onboarding-highlight",
          "target-highlight"
        );
      }
    };
  }, [currentStep, isOpen, highlightedElement]);

  const handleNext = () => {
    if (currentStep < steps.length - 1) {
      setCurrentStep(currentStep + 1);
    } else {
      // Last step, close the onboarding
      completeOnboarding();
    }
  };

  const handlePrevious = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const completeOnboarding = () => {
    // Mark onboarding as completed
    localStorage.setItem("onboardingShown", "true");
    setHasSeenOnboarding(true);
    setIsOpen(false);
    setHasCompletedOnboarding(true);

    // Clean up any lingering highlights
    const highlightedElements = document.querySelectorAll(
      ".onboarding-highlight, .target-highlight"
    );
    highlightedElements.forEach((el) => {
      el.classList.remove("onboarding-highlight", "target-highlight");
    });

    document.body.classList.remove("onboarding-active");
  };

  return (
    <>
      {!hasSeenOnboarding && (
        <Dialog
          open={isOpen}
          onOpenChange={(open) => {
            if (!open) completeOnboarding();
            setIsOpen(open);
          }}
        >
          <DialogContent className="sm:max-w-md onboarding-dialog">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                {steps[currentStep].icon && (
                  <div className="text-primary">{steps[currentStep].icon}</div>
                )}
                {steps[currentStep].title}
              </DialogTitle>
              <DialogDescription>
                {steps[currentStep].description}
              </DialogDescription>
            </DialogHeader>
            <div className="flex items-center justify-center gap-1 my-2">
              {steps.map((_, index) => (
                <div
                  key={index}
                  className={`h-1.5 rounded-full transition-all ${
                    index === currentStep ? "w-6 bg-primary" : "w-2 bg-muted"
                  }`}
                />
              ))}
            </div>
            <DialogFooter className="sm:justify-between">
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={completeOnboarding}
                  className="text-muted-foreground"
                >
                  Skip
                </Button>
                {currentStep > 0 && (
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handlePrevious}
                  >
                    Back
                  </Button>
                )}
              </div>
              <Button onClick={handleNext} className="flex items-center">
                {currentStep === steps.length - 1 ? (
                  <>
                    Done <Check className="ml-1 h-4 w-4" />
                  </>
                ) : (
                  <>
                    Next <ArrowRight className="ml-1 h-4 w-4" />
                  </>
                )}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
