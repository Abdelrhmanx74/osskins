/**
 * Download API - High-performance skin download using the new LeagueSkins repository
 *
 * This module provides TypeScript bindings for the Rust download backend
 * which supports:
 * - Parallel batch downloads with up to 8 concurrent connections
 * - Progress tracking with speed calculation
 * - Automatic retry with exponential backoff
 * - Cancellation support
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  useDownloadsStore,
  type BatchDownloadProgress,
  type SkinDownloadRequest,
} from "../store/downloads";
import { buildSkinDownloadUrl } from "../data-utils";

// Event names matching Rust backend
const DOWNLOAD_PROGRESS_EVENT = "download-progress";
const BATCH_DOWNLOAD_PROGRESS_EVENT = "batch-download-progress";

/**
 * Result from batch download operation
 */
export interface BatchDownloadResult {
  batchId: string;
  successful: string[];
  failed: [string, string][]; // [item_id, error]
  totalBytes: number;
  elapsedSecs: number;
}

/**
 * Download a single skin by ID
 * Uses the new LeagueSkins repository structure: skins/{champion_id}/{skin_id}/{skin_id}.zip
 */
export async function downloadSkinById(
  championId: number,
  skinId: number,
  championName: string,
  fileName: string,
  chromaId?: number,
  formId?: number,
): Promise<string> {
  return invoke<string>("download_skin_by_id", {
    championId,
    skinId,
    chromaId,
    formId,
    championName,
    fileName,
  });
}

/**
 * Download multiple skins in parallel using batch download
 * This is the most efficient way to download multiple skins
 */
export async function batchDownloadSkins(
  requests: SkinDownloadRequest[],
): Promise<BatchDownloadResult> {
  return invoke<BatchDownloadResult>("batch_download_skins", { requests });
}

/**
 * Check if a skin file exists in the LeagueSkins repository
 */
export async function checkSkinExists(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): Promise<boolean> {
  return invoke<boolean>("check_skin_exists", {
    championId,
    skinId,
    chromaId,
    formId,
  });
}

/**
 * Get the file size of a skin before downloading
 */
export async function getSkinFileSize(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): Promise<number | null> {
  return invoke<number | null>("get_skin_file_size", {
    championId,
    skinId,
    chromaId,
    formId,
  });
}

/**
 * Cancel an active download by ID
 */
export async function cancelDownload(id: string): Promise<boolean> {
  return invoke<boolean>("cancel_download", { id });
}

/**
 * Cancel an active batch download
 */
export async function cancelBatchDownload(batchId: string): Promise<boolean> {
  return invoke<boolean>("cancel_batch_download", { batchId });
}

/**
 * Subscribe to download progress events
 * Returns an unsubscribe function
 */
export async function subscribeToDownloadProgress(
  callback: (progress: BatchDownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<BatchDownloadProgress>(BATCH_DOWNLOAD_PROGRESS_EVENT, (event) => {
    callback(event.payload);
    // Also update the store
    // eslint-disable-next-line @typescript-eslint/no-unsafe-call
    useDownloadsStore.getState().updateBatch(event.payload);
  });
}

/**
 * Subscribe to individual download progress events
 */
export async function subscribeToSingleDownloadProgress(): Promise<UnlistenFn> {
  return listen(DOWNLOAD_PROGRESS_EVENT, (event) => {
    const payload = event.payload as {
      id: string;
      status: string;
      url: string;
      category: string;
      downloaded?: number;
      total?: number;
      speed?: number;
      championName?: string;
      fileName?: string;
      destPath?: string;
      error?: string;
    };

    useDownloadsStore.getState().upsert({
      id: payload.id,
      url: payload.url,
      category: payload.category as "skin" | "data" | "tools" | "misc" | "batch",
      status: payload.status as "queued" | "downloading" | "completed" | "failed" | "canceled",
      downloaded: payload.downloaded,
      total: payload.total,
      speed: payload.speed,
      championName: payload.championName,
      fileName: payload.fileName,
      destPath: payload.destPath,
      error: payload.error,
    });
  });
}

/**
 * Create a download request for a skin
 * Helper function to construct SkinDownloadRequest objects
 */
export function createSkinDownloadRequest(
  championId: number,
  skinId: number,
  championName: string,
  skinName: string,
  chromaId?: number,
  formId?: number,
): SkinDownloadRequest {
  // Generate a safe filename
  const sanitizedName = skinName
    .toLowerCase()
    .trim()
    .replace(/[/\\:?*"<>|()' ]+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "");

  let fileName: string;
  if (chromaId) {
    fileName = `${sanitizedName}_chroma_${chromaId}.zip`;
  } else if (formId) {
    fileName = `${sanitizedName}_form_${formId}.zip`;
  } else {
    fileName = `${sanitizedName}.zip`;
  }

  return {
    championId,
    skinId,
    chromaId,
    formId,
    championName: championName.toLowerCase().replace(/[^a-z0-9]/g, "_"),
    fileName,
  };
}

/**
 * Get the URL for a skin download (for preview/validation)
 */
export function getSkinUrl(
  championId: number,
  skinId: number,
  chromaId?: number,
  formId?: number,
): string {
  return buildSkinDownloadUrl(championId, skinId, chromaId, formId);
}

/**
 * Batch download skins for a champion
 * Convenience function that creates requests and downloads in parallel
 */
export async function downloadChampionSkins(
  championId: number,
  championName: string,
  skins: Array<{
    id: number;
    name: string;
    chromas?: Array<{ id: number; name: string }>;
  }>,
): Promise<BatchDownloadResult> {
  const requests: SkinDownloadRequest[] = [];

  for (const skin of skins) {
    // Add main skin
    requests.push(
      createSkinDownloadRequest(championId, skin.id, championName, skin.name),
    );

    // Add chromas if present
    if (skin.chromas) {
      for (const chroma of skin.chromas) {
        requests.push(
          createSkinDownloadRequest(
            championId,
            skin.id,
            championName,
            skin.name,
            chroma.id,
          ),
        );
      }
    }
  }

  return batchDownloadSkins(requests);
}
