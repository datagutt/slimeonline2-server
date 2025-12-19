I have this old GameMaker: Studio 1.x game client for a MMO, but i am missing the original server files.
Can you plan a full rust reimplementation of ../slime2_decompile.gmx/ (this is a folder with a decompiled client version)
THey are using a weird network library called 39dll that i want you to replicate in rust as well, or replace it with compatible code.
This is found in ../39DLL

../slime2_mod_tool_decompile.gmx/ folder contains the moderation tools for the game.
../SlimeOnline2_Client_Modified/ contains the full built client

I want a full modern rewrite of the server. Create the plan, with detailed docs divided into folders and multiple files, that can be used as documentation for a future LLM agent to implement this SERVER IMPLEMENTATION in rust.
we want it to be as secure as possible without ever doing client-side changes.
It should be fully featured implementation with no shortcuts or todos
DO NOT ADD ANY CODE FOR "MORE SECURE" OR TLS ANYTHING LIKE THAT. THIS IS A LEGACY CLIENT, WE CANT CHANGE IT. THIS IS FOR LEGACY PRIVATE SERVER EMU ONLY

Use existing docs in README.md and docs/ for reference
