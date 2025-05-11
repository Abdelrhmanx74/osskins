import { NextRequest } from "next/server";
import { invoke } from "@tauri-apps/api/core";
import { CustomSkin } from "@/lib/types";

// Configure route for static export
export const dynamic = "force-dynamic";
export const runtime = "edge";

export async function GET() {
  try {
    const skins = await invoke<CustomSkin[]>("get_custom_skins");
    return Response.json(skins);
  } catch (error) {
    console.error("Failed to load custom skins:", error);
    return Response.json(
      { error: "Failed to load custom skins" },
      { status: 500 }
    );
  }
}

interface UploadRequest {
  championId: number;
  skinName: string;
}

export async function POST(request: NextRequest) {
  try {
    const { championId, skinName } = (await request.json()) as UploadRequest;
    const newSkin = await invoke<CustomSkin>("upload_custom_skin", {
      championId,
      skinName,
    });
    return Response.json(newSkin);
  } catch (error) {
    console.error("Failed to upload custom skin:", error);
    return Response.json(
      { error: "Failed to upload custom skin" },
      { status: 500 }
    );
  }
}

interface DeleteRequest {
  skinId: string;
}

export async function DELETE(request: NextRequest) {
  try {
    const { skinId } = (await request.json()) as DeleteRequest;
    await invoke("delete_custom_skin", { skinId });
    return Response.json({ success: true });
  } catch (error) {
    console.error("Failed to delete custom skin:", error);
    return Response.json(
      { error: "Failed to delete custom skin" },
      { status: 500 }
    );
  }
}
