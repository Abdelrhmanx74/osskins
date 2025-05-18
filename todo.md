# Back End ---------------------

- [x] Fix build issues
- [x] State for user selects
- [-] Cache update with github / the json's update when there is a git commit
- [x] Refine LCU integration: Detect game state (champ select, in-game) for context-aware actions (e.g., auto-inject on game start, pre-game pop up).
- [x] Handle errors gracefully and provide user feedback on a prod level.
- [x] Custom skins tab. as in a whole different app tab with a dialog that opens uploading the skin file and adding its champion. and the skin name, etc...
- [-] Inject skins on reconnect

- [x] Optimize performance for large champion/skin datasets.
- [x] Ensure `mod-tools.exe` is compiled on build and not placed manually
- [x] app cosmetics and name, icon, etc...
- [x] Consider using a structured format (like JSON) for configuration instead of `league_path.txt` if more settings are planned.
- [x] It should not stop the injecting if the user closed the game (waiting to reconnect) it should close when it turn from in game to lobby, etc..
- [x] The terminals that opens!! it should not!
- [x] Handle game modes (arena, swift play)
- [x] Better injection error handling and cleaning cases not just at the end of a game
- [x] Run in background in system tray
- [ ] Custom Misc support
- [ ] Aram support
- [ ] Simultaneously use two files in the custom skin
- [ ] Test the cleaning function
- [ ] Make the injection status work till the end of the game
- [x] Refactor the command & injection files to be around ~500 each
- [ ] Implement git-like feature so the user doesn't have to update manually
- [ ] Check for better injection structure ways
- [ ] Checking for messages doesn't work lcu shit...
- [ ] Force injection button

# Front End ---------------------

- [x] Better front-end code
- [x] Logical loading/stale state
- [ ] Change the chroma selector indicator UI
- [x] Add Theming
- [x] All contexts should be zustand or react not both at the same time

# UX ---------------------

- [x] Favorites champs logic
- [x] Add search/filtering capabilities for the champion/skin list.
- [-] Animations baby!
