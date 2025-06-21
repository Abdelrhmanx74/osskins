import { NextRequest } from "next/server";
import { invoke } from "@tauri-apps/api/core";
import { Champion } from "@/lib/types";

// Configure route for static export
export const dynamic = "force-dynamic";
export const runtime = "edge";
export const preferredRegion = "auto";

export async function GET(request: NextRequest) {
  // Instead of checking headers which can be unreliable in edge runtime,
  // we'll add proper error handling for the Tauri invoke calls
  try {
    // First check if we have champion data
    const hasData = await invoke<boolean>("check_champions_data");

    if (!hasData) {
      return Response.json(
        { error: "No champion data found. Please run the data update first." },
        { status: 404 }
      );
    }

    const data = await invoke<string>("get_champion_data", {
      championId: 0,
    });

    if (!data) {
      return Response.json(
        { error: "No champion data available" },
        { status: 404 }
      );
    }

    const championsData = JSON.parse(data) as Champion[];
    if (!Array.isArray(championsData) || championsData.length === 0) {
      return Response.json(
        { error: "No champions found in data" },
        { status: 404 }
      );
    }

    return new Response(JSON.stringify(championsData), {
      status: 200,
      headers: {
        "Content-Type": "application/json",
      },
    });
  } catch (error) {
    // If the error is related to Tauri not being available (in SSR/non-Tauri context)
    if (error instanceof Error && error.message.includes("not available")) {
      return Response.json(
        { error: "This API is only available in the Tauri app context" },
        { status: 400 }
      );
    }

    console.error("Failed to load champions:", error);
    return Response.json(
      { error: "Failed to load champions" },
      { status: 500 }
    );
  }
}

export async function POST(request: NextRequest) {
  try {
    await invoke("delete_champions_cache");
    const result = await invoke<boolean>("check_champions_data");
    return Response.json({ success: true, hasData: result });
  } catch (error) {
    console.error("Failed to update champions:", error);
    return Response.json(
      { error: "Failed to update champions" },
      { status: 500 }
    );
  }
}
