"use client";

import React, { useMemo, useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { ChevronDown } from "lucide-react";
import { useGameStore } from "@/lib/store";
import { ManualInjectionPreviewDialog } from "@/components/ManualInjectionPreviewDialog";
import { useI18n } from "@/lib/i18n";
import { toast } from "sonner";
import { manualInjectionApi } from "@/lib/api/manual-injection";
import { listen } from "@tauri-apps/api/event";

export function ButtonInjection() {
  const { t } = useI18n();
  const manualInjectionMode = useGameStore((s) => s.manualInjectionMode);
  const manualSelectedSkins = useGameStore((s) => s.manualSelectedSkins);
  const selectedMiscItems = useGameStore((s) => s.selectedMiscItems);

  const [showPreviewDialog, setShowPreviewDialog] = useState(false);
  const [isInjecting, setIsInjecting] = useState(false);

  // Listen for manual injection status updates from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen<string>("manual-injection-status", (event) => {
        const status = event.payload;

        if (
          status === "waiting" ||
          status === "injecting" ||
          status === "success"
        ) {
          setIsInjecting(true);
        } else if (status === "stopped" || status === "error") {
          setIsInjecting(false);
        }
      });
    };

    void setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const totalSelectedItems = useMemo(() => {
    let total = manualSelectedSkins.size;
    for (const ids of selectedMiscItems.values()) {
      total += ids.length;
    }
    return total;
  }, [manualSelectedSkins, selectedMiscItems]);

  const handleManualInject = async () => {
    try {
      // Prepare skins data for backend
      const skinsArray = Array.from(manualSelectedSkins.values()).map(
        (skin) => ({
          champion_id: skin.championId,
          skin_id: skin.skinId,
          chroma_id: skin.chromaId,
          fantome: skin.fantome,
        }),
      );

      // Prepare misc items data for backend
      const miscItemsArray: Array<{
        id: string;
        name: string;
        item_type: string;
        fantome_path: string;
      }> = [];

      // Get all selected misc items from the store
      const allMiscItems = useGameStore.getState().miscItems;
      selectedMiscItems.forEach((itemIds, type) => {
        const typeItems = allMiscItems.get(type) ?? [];
        itemIds.forEach((itemId) => {
          const item = typeItems.find((i) => i.id === itemId);
          if (item) {
            miscItemsArray.push(item);
          }
        });
      });

      // Call backend to start manual injection
      await manualInjectionApi.startManualInjection(skinsArray, miscItemsArray);

      // Backend will manage the injection state
      toast.success(t("manual_injection.started"));
    } catch (error) {
      console.error("Failed to start manual injection:", error);
      toast.error(t("manual_injection.start_failed"));
    }
  };

  const handleStopManualInjection = async () => {
    try {
      // Call backend to stop manual injection
      await manualInjectionApi.stopManualInjection();

      // Backend will clean up and reset state
      toast.success(t("manual_injection.stopped"));
    } catch (error) {
      console.error("Failed to stop manual injection:", error);
      toast.error(t("manual_injection.stop_failed"));
    }
  };

  return (
    <div className="flex items-center gap-0">
      <Button
        onClick={() => {
          if (isInjecting) void handleStopManualInjection();
          else void handleManualInject();
        }}
        size="sm"
        className="rounded-l-full rounded-r-none border-r-0"
        variant={isInjecting ? "destructive" : "default"}
        disabled={!isInjecting && totalSelectedItems === 0}
      >
        {isInjecting
          ? t("manual_injection.stop")
          : t("manual_injection.inject")}
      </Button>

      <Button
        onClick={() => {
          setShowPreviewDialog(true);
        }}
        size="sm"
        className="rounded-r-full rounded-l-none"
        variant={isInjecting ? "destructive" : "outline"}
        disabled={totalSelectedItems === 0}
      >
        {totalSelectedItems > 0 && !isInjecting && (
          <p className="text-sm">{totalSelectedItems}</p>
        )}
        <ChevronDown className="h-4 w-4" />
      </Button>

      <ManualInjectionPreviewDialog
        open={showPreviewDialog}
        onOpenChange={setShowPreviewDialog}
      />
    </div>
  );
}

export default ButtonInjection;
