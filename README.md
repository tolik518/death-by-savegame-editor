# Death by Savegame Editor

Savegame editor for the game "Death by Daylight". 
It allows players to fully modify their save files.  

# Status: WIP
There is already a proof of concept python script that can decrypt and encrypt save files.
You can edit the save file in a text editor after decryption.

Save File Location:
- Linux: `~/.local/share/Terrible Toybox/Death by Scrolling/save
- Windows: `C:\Users\<YourUsername>\AppData\Local\Terrible Toybox\Death by Scrolling\save.bin` (unverified location)
- MacOS: `~/Library/Application Support/Terrible Toybox/Death by Scrolling/save.bin` (unverified location)

Linux: 
1. Clone/Download this repository
2. Go to the `scripts` folder and open your Terminal
3. run `python3 dbs_codec.py decrypt "/home/tolik/.local/share/Terrible Toybox/Death by Scrolling/save.bin" payload.hocon`
4. Change some data in the `payload.hocon` file
5. run `python3 dbs_codec.py encrypt payload.hocon "/home/tolik/.local/share/Terrible Toybox/Death by Scrolling/save.bin"`

(!) YOU CAN LOSE ALL YOUR PROGRESS IF YOU MAKE A MISTAKE (!)
(!) YOU CAN GET BANNED FROM THE LEADERBOARD FOR USING MODIFIED SAVE FILES (!)





Quote by the game developer, Ron Gilbert:
> Save games are encrypted with unbreakable xor encryption. It could take you up to 256 attempts! Not sure that is even possible on modern computers.
