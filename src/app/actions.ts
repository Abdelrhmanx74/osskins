"use server";

import { revalidatePath } from "next/cache";
import { invoke } from "@tauri-apps/api/core";
import { CustomSkin, Champion } from "@/lib/types";

interface UploadResult {
  success: boolean;
  skin?: CustomSkin;
  error?: string;
}

interface DeleteResult {
  success: boolean;
  error?: string;
}

interface ChampionsResult {
  success: boolean;
  champions?: Champion[];
  error?: string;
}

interface ChampionUpdateResult {
  success: boolean;
  hasData?: boolean;
  error?: string;
}

export async function uploadSkin(
  championId: number,
  skinName: string
): Promise<UploadResult> {
  try {
    // The Tauri command will handle file selection on the native side
    const newSkin = await invoke<CustomSkin>("upload_custom_skin", {
      championId,
      skinName,
    });
    revalidatePath("/");
    return { success: true, skin: newSkin };
  } catch (error) {
    console.error("Failed to upload custom skin:", error);
    return { success: false, error: String(error) };
  }
}

export async function deleteSkin(skinId: string): Promise<DeleteResult> {
  try {
    await invoke("delete_custom_skin", { skinId });
    revalidatePath("/");
    return { success: true };
  } catch (error) {
    return { success: false, error: String(error) };
  }
}

export async function getChampions(): Promise<ChampionsResult> {
  try {
    // First check if we have champion data
    const hasData = await invoke<boolean>("check_champions_data");

    if (!hasData) {
      return {
        success: false,
        error: "No champion data found. Please run the data update first.",
      };
    }

    const data = await invoke<string>("get_champion_data", {
      championId: 0,
    });

    if (!data) {
      return {
        success: false,
        error: "No champion data available",
      };
    }

    const championsData = JSON.parse(data) as Champion[];
    if (!Array.isArray(championsData) || championsData.length === 0) {
      return {
        success: false,
        error: "No champions found in data",
      };
    }

    return { success: true, champions: championsData };
  } catch (error) {
    // If the error is related to Tauri not being available (in SSR/non-Tauri context)
    if (error instanceof Error && error.message.includes("not available")) {
      return {
        success: false,
        error: "This API is only available in the Tauri app context",
      };
    }

    console.error("Failed to load champions:", error);
    return {
      success: false,
      error: "Failed to load champions",
    };
  }
}

export async function updateChampions(): Promise<ChampionUpdateResult> {
  try {
    await invoke("delete_champions_cache");
    const result = await invoke<boolean>("check_champions_data");
    return { success: true, hasData: result };
  } catch (error) {
    console.error("Failed to update champions:", error);
    return {
      success: false,
      error: "Failed to update champions",
    };
  }
}

export async function getCustomSkins(): Promise<{
  success: boolean;
  skins?: CustomSkin[];
  error?: string;
}> {
  try {
    const skins = await invoke<CustomSkin[]>("get_custom_skins");
    return { success: true, skins };
  } catch (error) {
    console.error("Failed to load custom skins:", error);
    return {
      success: false,
      error: "Failed to load custom skins",
    };
  }
}
