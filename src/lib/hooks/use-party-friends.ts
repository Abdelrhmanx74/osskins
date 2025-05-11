import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export interface Friend {
  name: string;
  id: string;
  availability: "online" | "away" | "offline" | "in-game";
  game?: string;
  inParty?: boolean;
}

export function usePartyFriends(leaguePath: string | null) {
  const [friends, setFriends] = useState<Friend[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const fetchFriends = useCallback(async () => {
    setIsLoading(true);
    if (!leaguePath) {
      toast.error(
        "League path not set. Please set your League of Legends path in settings."
      );
      setIsLoading(false);
      return;
    }
    try {
      const lcuFriends = await invoke<any[]>("get_lcu_friends", {
        app: window.__TAURI__,
        leaguePath,
      });
      const mappedFriends: Friend[] = lcuFriends.map((friend) => ({
        name: friend.name,
        id: friend.id,
        availability:
          friend.availability === "chat"
            ? "online"
            : friend.availability === "dnd"
            ? "away"
            : friend.availability === "away"
            ? "away"
            : friend.availability === "offline"
            ? "offline"
            : friend.availability === "mobile"
            ? "away"
            : "online",
        game: friend.gameTag || undefined,
      }));
      setFriends(mappedFriends);
    } catch (error) {
      // fallback to mock friends
      setFriends([
        { name: "SummonerOne", id: "friend1", availability: "online" },
        {
          name: "SummonerTwo",
          id: "friend2",
          availability: "in-game",
          game: "League of Legends",
        },
        { name: "SummonerThree", id: "friend3", availability: "away" },
      ]);
      toast.error("League client not detected. Using demo friends list.");
    } finally {
      setIsLoading(false);
    }
  }, [leaguePath]);

  return { friends, isLoading, fetchFriends };
}
