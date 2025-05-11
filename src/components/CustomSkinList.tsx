"use client";

import React, { useState, Suspense } from "react";
import { useChampions } from "@/lib/hooks/use-champions";
import { CustomSkinItem } from "./CustomSkinItem";
import { Button } from "./ui/button";
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
import { Skeleton } from "./ui/skeleton";
import { useCustomSkins } from "@/lib/hooks/use-custom-skins";

interface CustomSkinListProps {
  championId: number | null;
}

function CustomSkinLoading() {
  return (
    <div className="size-full space-y-3 px-20 py-10">
      {Array.from({ length: 3 }).map((_, i) => (
        <div
          key={i}
          className="w-full py-6 px-1 bg-primary/20 gap-0 rounded-lg overflow-hidden animate-pulse"
        >
          <div className="p-2">
            <Skeleton className="size-8" />
          </div>
          <div className="flex justify-between gap-2 px-2 items-center w-full">
            <Skeleton className="h-6 w-32" />
            <Skeleton className="size-8 rounded-full" />
          </div>
        </div>
      ))}
    </div>
  );
}

export function CustomSkinList({ championId }: CustomSkinListProps) {
  const { champions } = useChampions();
  const { customSkins, uploadCustomSkin, deleteCustomSkin } = useCustomSkins();
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [skinName, setSkinName] = useState("");
  const [isUploading, setIsUploading] = useState(false);

  const champion = championId
    ? champions.find((c) => c.id === championId)
    : null;
  const championCustomSkins =
    championId !== null ? customSkins.get(championId) ?? [] : [];

  const handleAddNewSkin = () => {
    if (!championId) {
      toast.error("Please select a champion first");
      return;
    }
    setIsDialogOpen(true);
  };

  const handleUploadSkin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!championId) return;

    setIsUploading(true);
    try {
      const newSkin = await uploadCustomSkin(championId, skinName);
      if (newSkin) {
        setIsDialogOpen(false);
        setSkinName("");
        toast.success("Custom skin uploaded successfully");
      }
    } catch (error) {
      toast.error("Failed to upload custom skin");
      console.error(error);
    } finally {
      setIsUploading(false);
    }
  };

  const handleDeleteSkin = async (skinId: string): Promise<boolean> => {
    try {
      const success = await deleteCustomSkin(skinId);
      if (success) {
        toast.success("Skin deleted successfully");
        return true;
      } else {
        toast.error("Failed to delete skin");
        return false;
      }
    } catch (error) {
      toast.error("An error occurred while deleting the skin");
      return false;
    }
  };

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
    <Suspense fallback={<CustomSkinLoading />}>
      <div className="size-full space-y-3 px-20 py-10">
        {championCustomSkins.map((skin) => (
          <CustomSkinItem
            key={skin.id}
            skin={skin}
            onDelete={handleDeleteSkin}
          />
        ))}

        {championCustomSkins.length === 0 && (
          <div className="flex flex-col items-center mt-8">
            <p className="text-muted-foreground mb-4">
              No custom skins found for {champion?.name ?? "this champion"}.
            </p>
          </div>
        )}

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

            <form onSubmit={handleUploadSkin}>
              <div className="grid gap-4 py-4">
                <div className="flex flex-col gap-2">
                  <Label htmlFor="skinName">Skin Name</Label>
                  <Input
                    id="skinName"
                    name="skinName"
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
                    setSkinName("");
                  }}
                  disabled={isUploading}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  disabled={isUploading || !skinName.trim()}
                >
                  {isUploading ? "Uploading..." : "Upload Skin"}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      </div>
    </Suspense>
  );
}
