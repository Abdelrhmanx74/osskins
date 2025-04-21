# Back End ---------------------

- [x] Fix build issues
- [x] State for user selects
- [ ] Cache update with github / the json's update when there is a git commit
- [x] Refine LCU integration: Detect game state (champ select, in-game) for context-aware actions (e.g., auto-inject on game start, pre-game pop up).
- [ ] Handle errors gracefully and provide user feedback on a prod level.

- [-] Optimize performance for large champion/skin datasets.
- [x] Ensure `mod-tools.exe` is compiled on build and not placed manually
- [ ] app cosmetics and name, icon, etc...
- [x] Consider using a structured format (like JSON) for configuration instead of `league_path.txt` if more settings are planned.
- [x?] It shouldnt stop the injecting if the user closed the game (waiting to reconnect) it should close when it turn from in game to lobby, etc..
- [ ] The terminals that opens!! it shouldnt!

# Front End ---------------------

- [ ] Better front-end code
- [ ] Logical loading/stale state

- [ ] Add Theming
- [ ] All contexts should be zustand or react not both at the same time

# UX ---------------------

- [x] Favorites champs logic
- [x] Add search/filtering capabilities for the champion/skin list.
- [ ] Animations baby!
