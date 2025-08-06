// Party Mode Types - matches Rust backend types

export interface PartyModeConfig {
  paired_friends: PairedFriend[];
  notifications: boolean;
}

export interface PairedFriend {
  summoner_id: string;
  summoner_name: string;
  display_name: string;
  paired_at: number;
  share_enabled: boolean;
}

export interface FriendInfo {
  summoner_id: string;
  summoner_name: string;
  display_name: string;
  is_online: boolean;
  availability?: string;
  puuid: string;
  pid: string;
}

export interface PartyModeMessage {
  message_type: string;
  data: any;
}

export interface SkinShare {
  from_summoner_id: string;
  from_summoner_name: string;
  champion_id: number;
  skin_id: number;
  skin_name: string; // Add skin name field
  chroma_id?: number;
  fantome_path?: string;
  timestamp: number;
}

// Frontend-specific types
export interface Friend {
  id: string;
  name: string;
  display_name: string;
  is_online: boolean;
  availability?: string;
}
