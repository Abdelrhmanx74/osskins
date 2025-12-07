import { useI18n } from "@/lib/i18n";
import React, { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { toast } from "sonner";
import { Users, UserPlus, UserMinus, RefreshCcw } from "lucide-react";
import { DropdownMenuItem } from "./ui/dropdown-menu";
import {
  partyModeApi,
  getStatusColor,
  getStatusText,
} from "@/lib/api/party-mode";
import { usePartyModeStore } from "@/lib/store/party-mode";
import type {
  FriendInfo,
  PairedFriend,
  SkinShare,
} from "@/lib/types/party-mode";
import { useGameStore } from "@/lib/store";

export default function PartyModeDialog() {
  const [isOpen, setIsOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const { t } = useI18n();
  const [selectedTab, setSelectedTab] = useState("connect");
  const [searchTerm, setSearchTerm] = useState("");
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);

  // Real data from backend
  const [friends, setFriends] = useState<FriendInfo[]>([]);

  // Use Zustand store for paired friends
  const pairedFriends = usePartyModeStore((s) => s.pairedFriends);
  const manualInjectionMode = useGameStore((s) => s.manualInjectionMode);

  // Set up global event listeners that work even when dialog is closed
  useEffect(() => {
    // Listen for custom event to open dialog
    const handleOpenDialog = () => {
      setIsOpen(true);
    };
    document.addEventListener("open-party-mode-dialog", handleOpenDialog);

    // Return synchronous cleanup function
    return () => {
      document.removeEventListener("open-party-mode-dialog", handleOpenDialog);
    };
  }, []);

  // Load data when dialog opens
  useEffect(() => {
    if (isOpen) {
      void loadFriends();
      void loadSettings();
    }
  }, [isOpen]);

  const loadFriends = async () => {
    try {
      const friendsList = await partyModeApi.getFriends();
      setFriends(friendsList);
    } catch (error) {
      console.error("Failed to load friends:", error);
      toast.error("Failed to load friends list");
    }
  };

  const loadSettings = async () => {
    try {
      const settings = await partyModeApi.getSettings();
      setNotificationsEnabled(settings.notifications);
    } catch (error) {
      console.error("Failed to load settings:", error);
      // Don't show toast for this as it's not critical
    }
  };

  // Helper functions for UI
  const getStatusColorForFriend = (friend: FriendInfo) => {
    return getStatusColor(friend.availability, friend.is_online);
  };

  const getStatusTextForFriend = (friend: FriendInfo) => {
    return getStatusText(friend.availability, friend.is_online);
  };

  const filterFriends = (searchTerm: string) => {
    return friends.filter(
      (friend) =>
        friend.summoner_name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        friend.display_name.toLowerCase().includes(searchTerm.toLowerCase()),
    );
  };

  // Handlers for UI
  const handleLoadFriendsList = async () => {
    setIsLoading(true);
    try {
      await loadFriends();
    } finally {
      setIsLoading(false);
    }
  };

  const handleAddFriend = async (friendSummonerId: string) => {
    console.log(
      "[DEBUG] handleAddFriend called with friendSummonerId:",
      friendSummonerId,
    );
    setIsLoading(true);
    try {
      console.log("[DEBUG] About to call partyModeApi.addPartyFriend...");
      await partyModeApi.addPartyFriend(friendSummonerId);
      console.log("[DEBUG] Successfully added friend to party mode");
      toast.success("Friend added to party mode!");
    } catch (error) {
      console.error("Failed to add friend:", error);
      const errorMessage =
        error instanceof Error ? error.message : "Failed to add friend";
      toast.error(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  const removeFriend = async (friendSummonerId: string) => {
    try {
      await partyModeApi.removePairedFriend(friendSummonerId);
      toast.success("Friend removed successfully");
    } catch (error) {
      console.error("Failed to remove friend:", error);
      toast.error("Failed to remove friend");
    }
  };

  const toggleNotifications = async (checked: boolean) => {
    try {
      await partyModeApi.updateSettings(checked);
      setNotificationsEnabled(checked);
    } catch (error) {
      console.error("Failed to update notifications setting:", error);
      toast.error("Failed to update settings");
    }
  };

  const filteredFriends = filterFriends(searchTerm);

  // If manual injection mode is enabled, render a disabled menu item instead of the full dialog
  if (manualInjectionMode) {
    return (
      <DropdownMenuItem
        onSelect={(e) => {
          e.preventDefault();
        }}
        // visually muted and unselectable
        className="opacity-50 pointer-events-none"
      >
        <Users className="h-4 w-4" />
        {t("party.mode")}
      </DropdownMenuItem>
    );
  }

  return (
    <>
      <Dialog open={isOpen} onOpenChange={setIsOpen}>
        <DialogTrigger asChild>
          <DropdownMenuItem
            onSelect={(e) => {
              e.preventDefault();
            }}
          >
            <Users className="h-4 w-4" />
            {t("party.mode")}
          </DropdownMenuItem>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Users className="h-5 w-5" />
              {t("party.mode")}
            </DialogTitle>
          </DialogHeader>

          <Tabs value={selectedTab} onValueChange={setSelectedTab}>
            <TabsList className="grid w-full grid-cols-3">
              <TabsTrigger value="connect">{t("friends.label")}</TabsTrigger>
              <TabsTrigger value="connected">
                {t("friends.connected")} ({pairedFriends.length})
              </TabsTrigger>
              <TabsTrigger value="settings">{t("settings.title")}</TabsTrigger>
            </TabsList>

            <TabsContent value="connect" className="space-y-4">
              <div className="space-y-4">
                <div className="flex gap-2">
                  <Input
                    placeholder={t("party.search_placeholder")}
                    value={searchTerm}
                    onChange={(e) => {
                      setSearchTerm(e.target.value);
                    }}
                    className="flex-1"
                  />
                  <Button
                    onClick={() => {
                      void handleLoadFriendsList();
                    }}
                    disabled={isLoading}
                    variant={"ghost"}
                    size="icon"
                  >
                    <RefreshCcw className="h-4 w-4" />
                  </Button>
                </div>

                <div className="space-y-2 max-h-96 overflow-y-auto">
                  {isLoading ? (
                    <div className="text-center py-8">
                      <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto"></div>
                      <p className="text-sm text-muted-foreground mt-2">
                        {
                          t(
                            "loading.champions_data",
                          ) /* reuse a loading key; consider dedicated key if needed */
                        }
                      </p>
                    </div>
                  ) : filteredFriends.length === 0 ? (
                    <div className="text-center py-8">
                      <p className="text-muted-foreground">
                        {t("friends.no_friends")}
                      </p>
                    </div>
                  ) : (
                    filteredFriends.map((friend) => {
                      const pairedFriend = pairedFriends.find(
                        (cf) =>
                          String(cf.summoner_id) === String(friend.summoner_id),
                      );
                      const isConnected = !!pairedFriend;

                      return (
                        <div
                          key={friend.summoner_id}
                          className="flex items-center justify-between p-3 border rounded-lg"
                        >
                          <div className="flex items-center gap-3">
                            <div
                              className={`w-3 h-3 rounded-full ${getStatusColorForFriend(
                                friend,
                              )}`}
                            ></div>
                            <div>
                              <p className="font-medium">
                                {friend.display_name}
                              </p>
                              <p className="text-sm text-muted-foreground">
                                {getStatusTextForFriend(friend)}
                              </p>
                            </div>
                          </div>
                          <Switch
                            checked={isConnected}
                            onCheckedChange={(checked) => {
                              if (checked) {
                                void handleAddFriend(friend.summoner_id);
                              } else {
                                void removeFriend(friend.summoner_id);
                              }
                            }}
                            disabled={isLoading}
                          />
                        </div>
                      );
                    })
                  )}
                </div>
              </div>
            </TabsContent>

            <TabsContent value="connected" className="space-y-4">
              <div>
                {pairedFriends.length === 0 ? (
                  <div className="text-center py-8">
                    <Users className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                    <p className="text-muted-foreground">
                      {t("friends.no_friends")}
                    </p>
                    <p className="text-sm text-muted-foreground mt-2">
                      {t("party.search_placeholder")}
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {pairedFriends.map((friend) => (
                      <div
                        key={friend.summoner_id}
                        className="flex items-center justify-between p-3 border rounded-lg"
                      >
                        <div className="flex items-center gap-3">
                          <div className="w-3 h-3 rounded-full bg-green-500"></div>
                          <div>
                            <p className="font-medium">{friend.display_name}</p>
                            <p className="text-sm text-muted-foreground">
                              {t("friends.connected")}{" "}
                              {new Date(friend.paired_at).toLocaleDateString()}
                            </p>
                          </div>
                        </div>
                        <Button
                          onClick={() => {
                            void removeFriend(friend.summoner_id);
                          }}
                          size="sm"
                          variant="destructive"
                        >
                          <UserMinus className="h-4 w-4 mr-2" />
                          {t("misc.delete")}
                        </Button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </TabsContent>

            <TabsContent value="settings" className="space-y-4">
              <Card>
                <CardContent className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div className="space-y-1">
                      <Label>{t("notifications.label")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("notifications.show_on_share")}
                      </p>
                    </div>
                    <Switch
                      checked={notificationsEnabled}
                      onCheckedChange={(checked) =>
                        void toggleNotifications(checked)
                      }
                    />
                  </div>
                </CardContent>
              </Card>
            </TabsContent>
          </Tabs>
        </DialogContent>
      </Dialog>
    </>
  );
}
