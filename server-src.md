Great news! I managed to get a copy of the server source code.
I want you to go through the docs/, making sure it is up-to-date with the actual implementation.
If something is wrong, change it. If additional info we can use to implement something is available, add it.
DO NOT write about how the docs "used to be" or reference the old/wrong docs in any way if they mismatch.
We want the docs/ folder to essentially not mention the "past docs" but just be up-to-date and correct.


After this, create a plan for updating the codebase to match the changes and implementing new stuff based on server code.
In particular, i know these are things we just estimated earlier beceause we didnt have server source:
- Shop prices
- Shop items
- Sell prices
- "Collectibles" spawn points (not slime points, but collectible items)


I see most data/configs for stuff is located in srvr_*/ folders

../slime2_server.decompile.gmx/ (decompiled server)
../slime2_decompile.gmx/ (decompiled client)
../slime2_mod_tool_decompile.gmx/ folder contains the moderation tools for the game.
../slime2_server (fully built server)
../SlimeOnline2_Client_Modified/ contains the full built client
../39DLL/ networking library source code