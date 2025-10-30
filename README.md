# Death by Savegame Editor

Savegame editor for the game "Death by Daylight". 
It allows players to fully modify their save files.  

# Status: WIP
There is already a proof of concept python script that can decrypt and encrypt save files.
You can edit the save file in a text editor after decryption.

# How to use

Save File Location:
- Linux: `~/.local/share/Terrible Toybox/Death by Scrolling/sav.bin`
- Windows: `C:\Users\<YourUsername>\AppData\Local\Terrible Toybox\Death by Scrolling\save.bin` (unverified location)
- MacOS: `~/Library/Application Support/Terrible Toybox/Death by Scrolling/save.bin` (unverified location)

⚠ MAKE A BACKUP BEFORE USING ⚠

1. Download the newest [release](https://github.com/tolik518/death-by-savegame-editor/releases)
2. Extract the zip file
3. Launch `dbs-gui`
4. Click **Browse** next to **Encrypted save** and select your save file
5. Click **Browse** next to **Output plaintext** to choose where to save the plaintext
6. Click **Decrypt Save File**
7. Edit the decrypted file with a text editor
8. Switch to **Encrypt** mode
9. Select the modified plaintext save file
10. Select the location to save the modified encrypted save file, it has to be named `save.bin`
11. Click **Encrypt Payload**
12. Replace your original save file with the modified one
13. Set `globalLeaderboards:` to `0` in the `Prefs.json`, which is in the same folder as the `save.bin`

⚠ YOU CAN GET **BANNED** FROM THE LEADERBOARD FOR USING MODIFIED SAVE FILES IF YOU HAVE SET `globalLeaderboards: 1` ⚠

Quote by the game developer, Ron Gilbert:
> Save games are encrypted with unbreakable xor encryption. It could take you up to 256 attempts! Not sure that is even possible on modern computers.
