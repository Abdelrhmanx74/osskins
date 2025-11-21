import axios, { type AxiosProgressEvent, type AxiosRequestConfig } from "axios";

export interface DownloadProgress {
  loaded: number;
  total: number;
  percent: number;
  speed?: number;
}

export interface DownloadOptions {
  onProgress?: (progress: DownloadProgress) => void;
  timeout?: number;
  signal?: AbortSignal;
}

/**
 * Downloads a file as a Uint8Array with optional progress tracking
 * Uses axios for better concurrency handling and progress tracking
 */
export async function downloadFile(
  url: string,
  options: DownloadOptions = {},
): Promise<Uint8Array> {
  const { onProgress, timeout = 30000, signal } = options;

  const config: AxiosRequestConfig = {
    url,
    method: "GET",
    responseType: "arraybuffer",
    timeout,
    signal,
    onDownloadProgress: onProgress
      ? (progressEvent: AxiosProgressEvent) => {
        const { loaded, total } = progressEvent;
        const percent = total ? (loaded / total) * 100 : 0;

        // Calculate speed (bytes per second)
        let speed: number | undefined;
        if (progressEvent.rate) {
          speed = progressEvent.rate;
        }

        onProgress({
          loaded,
          total: total ?? 0,
          percent,
          speed,
        });
      }
      : undefined,
  };

  try {
    const response = await axios(config);
    return new Uint8Array(response.data);
  } catch (error) {
    if (axios.isCancel(error)) {
      throw new Error("Download cancelled");
    }
    if (axios.isAxiosError(error)) {
      if (error.response) {
        throw new Error(
          `Download failed with status ${error.response.status}: ${error.response.statusText}`,
        );
      }
      if (error.request) {
        throw new Error("Download failed: No response received");
      }
    }
    throw error;
  }
}

/**
 * Downloads a file without progress tracking (simpler API)
 */
export async function downloadFileSimple(url: string): Promise<Uint8Array> {
  return downloadFile(url);
}

