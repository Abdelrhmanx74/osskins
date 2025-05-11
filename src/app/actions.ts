"use server";

import { revalidatePath } from "next/cache";
import { invoke } from "@tauri-apps/api/core";
import { CustomSkin } from "@/lib/types";

interface UploadResult {
  success: boolean;
  skin?: CustomSkin;
  error?: string;
}

interface DeleteResult {
  success: boolean;
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
