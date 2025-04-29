"use client";

import React, { useState } from "react";
import { useCustomSkins } from "@/lib/hooks/use-custom-skins";
import { CustomSkinItem } from "./CustomSkinItem";
import { Button } from "./ui/button";
import { useChampions } from "@/lib/hooks/use-champions";
import { Plus } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./ui/dialog";
import { Label } from "./ui/label";
import { Input } from "./ui/input";

interface CustomSkinListProps {
  championId: number | null;
}

export function CustomSkinList({ championId }: CustomSkinListProps) {
  const { customSkins, isLoading, error, deleteCustomSkin, uploadCustomSkin } =
    useCustomSkins();
  const { champions } = useChampions();

  // Add state for the dialog
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [skinName, setSkinName] = useState("");
  const [isUploading, setIsUploading] = useState(false);

  // Get the champion data if ID is provided
  const champion = championId
    ? champions.find((c) => c.id === championId)
    : null;

  // Filter skins for the selected champion
  const championCustomSkins =
    championId !== null ? customSkins.get(championId) ?? [] : [];

  // Handler for adding a new custom skin
  const handleAddNewSkin = () => {
    if (!championId) {
      toast.error("Please select a champion first");
      return;
    }

    // Set default skin name and open dialog
    const defaultName = champion
      ? `Custom ${champion.name} Skin`
      : "Custom Skin";
    setSkinName(defaultName);
    setIsDialogOpen(true);
  };

  // Handler for uploading the skin
  const handleUploadSkin = async () => {
    if (!championId) {
      toast.error("Please select a champion first");
      return;
    }

    if (!skinName.trim()) {
      toast.error("Please enter a skin name");
      return;
    }

    setIsUploading(true);
    try {
      const result = await uploadCustomSkin(championId, skinName);
      if (result) {
        toast.success(`Custom skin "${skinName}" added successfully`);
        setIsDialogOpen(false);
        resetForm();
      } else {
        toast.error("Failed to add custom skin. Please try again.");
      }
    } catch (err) {
      console.error("Error uploading skin:", err);
      toast.error("Error uploading skin. Please try again.");
    } finally {
      setIsUploading(false);
    }
  };

  // Reset the form
  const resetForm = () => {
    setSkinName("");
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full w-full">
        <p className="text-muted-foreground">Loading custom skins...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full w-full">
        <p className="text-destructive">Error loading custom skins: {error}</p>
        <Button
          variant="outline"
          className="mt-4"
          onClick={() => {
            window.location.reload();
          }}
        >
          Retry
        </Button>
      </div>
    );
  }

  // If no champion is selected
  if (!championId) {
    return (
      <div className="flex flex-col items-center justify-center h-full w-full p-4">
        <p className="text-muted-foreground">
          Please select a champion to view their custom skins.
        </p>
      </div>
    );
  }

  return (
    <>
      <div className="size-full space-y-3 px-20 py-10">
        {championCustomSkins.map((skin) => (
          <CustomSkinItem
            key={skin.id}
            skin={skin}
            onDelete={deleteCustomSkin}
          />
        ))}

        {championCustomSkins.length === 0 && (
          <div className="flex flex-col items-center mt-8">
            <p className="text-muted-foreground mb-4">
              No custom skins found for {champion?.name ?? "this champion"}.
            </p>
          </div>
        )}

        {/* Add skin button always shown at the end of the list */}
        <Button
          size={"lg"}
          variant="outline"
          className="w-full border-dashed py-6 mt-1 justify-start"
          onClick={handleAddNewSkin}
        >
          <Plus className="size-8 opacity-50" />
          <span className="text-lg font-medium">Add Custom Skin</span>
        </Button>

        <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Add Custom Skin</DialogTitle>
              <DialogDescription>
                Upload a custom skin file (.fantome) for{" "}
                {champion?.name ?? "your champion"}
              </DialogDescription>
            </DialogHeader>

            <div className="grid gap-4 py-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="name">Skin Name</Label>
                <Input
                  id="name"
                  value={skinName}
                  onChange={(e) => {
                    setSkinName(e.target.value);
                  }}
                  placeholder="Enter a name for this skin"
                />
              </div>
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setIsDialogOpen(false);
                  resetForm();
                }}
                disabled={isUploading}
              >
                Cancel
              </Button>
              <Button
                type="button"
                onClick={() => {
                  void handleUploadSkin();
                }}
                disabled={isUploading || !skinName.trim()}
              >
                {isUploading ? "Uploading..." : "Upload Skin"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </>
  );
}
