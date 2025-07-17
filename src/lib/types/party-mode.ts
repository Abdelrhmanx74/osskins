// Party Mode Types - matches Rust backend types

export interface PartyModeConfig {
  paired_friends: PairedFriend[];
  auto_share: boolean;
  notifications: boolean;
  received_skins: Record<string, ReceivedSkinData>;
}

export interface PairedFriend {
  summoner_id: string;
  summoner_name: string;
  display_name: string;
  paired_at: number;
}

export interface ReceivedSkinData {
  from_summoner_id: string;
  from_summoner_name: string;
  champion_id: number;
  skin_id: number;
  chroma_id?: number;
  fantome_path?: string;
  received_at: number;
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

export interface ConnectionRequest {
  from_summoner_id: string;
  from_summoner_name: string;
  timestamp: number;
}

export interface PartyModeMessage {
  message_type: string;
  data: any;
}

export interface PairingRequest {
  request_id: string;
  from_summoner_id: string;
  from_summoner_name: string;
  timestamp: number;
}

export interface PairingResponse {
  request_id: string;
  accepted: boolean;
  from_summoner_id: string;
  from_summoner_name: string;
  timestamp: number;
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
