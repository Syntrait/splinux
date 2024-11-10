# Splinux
A tool for splitting the screen on Linux, and passing inputs to them 

# Disclaimer
**This program might trigger anticheats for "automation", use at your own risk.**

# Usage

## Starting the program
In a 1920x1080 screen, to split the program horizontally, run both programs with

Launch your program (ex. librewolf) with

```gamescope -W 1920 -H 540 -- librewolf```

If the program is a game on Steam, add this to the command line arguments

```gamescope -W 1920 -H 540 -- %command%```

## Identifying the display ids
After launching your programs on separate gamescope sessions, you need to know their display ids

At the location "/tmp/.X11-unix/", you can see files with names like "X0", "X1", "X2", "X3", "X4", "X5", etc.
These files show what displays are currently open. The ones you should be looking for should be the recent ones with the biggest numbers.


## Identifying the device ids
If we run ```evtest```, we will be greeted with a list of devices currently connected to our system, along with their device ids.

<details>
  <summary>Example</summary>
  
  ```
/dev/input/event0:      Lid Switch
/dev/input/event1:      Power Button
/dev/input/event10:       USB Keyboard
/dev/input/event11:       USB Keyboard System Control
/dev/input/event12:       USB Keyboard Consumer Control
/dev/input/event13:     USB OPTICAL MOUSE
/dev/input/event14:     USB OPTICAL MOUSE  Keyboard
/dev/input/event25:     SEM HCT Keyboard
/dev/input/event256:    Telink Wireless Receiver
/dev/input/event26:     SEM HCT Keyboard Consumer Control
/dev/input/event27:     SEM HCT Keyboard System Control
/dev/input/event28:     Telink Wireless Receiver Mouse
/dev/input/event29:     Telink Wireless Receiver Consumer Control
/dev/input/event3:      Power Button
/dev/input/event30:     Telink Wireless Receiver System Control
/dev/input/event31:     Telink Wireless Receiver
/dev/input/event5:      ELAN1201:00 04F3:3098 Mouse
/dev/input/event6:      ELAN1201:00 04F3:3098 Touchpad
/dev/input/event7:      Asus Keyboard
/dev/input/event8:      Asus Keyboard
/dev/input/event9:      Asus Keyboard
  ```
</details>

We need to ignore things like "Consumer Control", "System Control", "Lid Switch", "Power button", and devices like keyboards belonging to mices.
In this case, I have 2 keyboards connected via USB (excluding the laptop keyboard) and 2 mice.

What we want from here are 25,28 and 10,13


## Launching Splinux
Launch Splinux. After launching Splinux, you will be greeted with 2 text boxes, and a "+" button.

Enter the display id you found in the box. If you found ```X30```, then type ```:30``` in the box.

Enter the device ids you found in the box. If you found ```/dev/input/event25``` and ```/dev/input/event28```, enter the box ```25,28```

### IMPORTANT: If you find yourself stuck, try disconnecting and reconnecting the devices, this will free the devices. Or, if you have a spare mouse or keyboard, you can click the desired client's "X" button

After you are ready, click the "+" button. This will grab the keyboard and mouse, and dedicate them to that display.


# How it works
## What is gamescope
[gamescope](https://github.com/ValveSoftware/gamescope) is a program that isolates the window with the system and runs it in its own window. It uses Wayland, but to be able to run X applications, it uses Xwayland, a compatibility layer.

When we pass the ```-W``` argument, we are passing the width, and ```-H``` for the height of our virtual screen size. After we run this command, a window of this size will appear.

The program that we will be running inside gamescope will think the the screen is 1920x540, and will run in a window of that size.

## What is Xwayland
Xwayland is a compatibility layer for Wayland. Wayland is still in development, and some programs are yet to adopt it, and still use X11. To run these applications in Wayland, we need a compatibility layer. That's what Xwayland is.
Xwayland will run a X server, and assign a display number to it (ex. DISPLAY=:30)

Every X server can only have **one** focused window, however, we can nest multiple X servers inside each other, and if we open multiple X servers, we can have multiple focused windows, and because of the isolation, they won't be affecting each other.

## What is bubblewrap (bwrap)
[bubblewrap](https://github.com/containers/bubblewrap) is a program that allows you to containerize, and sandbox programs.
While it's original purpose is to hide data from programs that doesn't need them, we use it to have 2 different save file locations.

It allows us to have 2 different save file locations. It's like creating a symbolic link, but it only exists for the program that we are running it for.

We will be utilizing the ```--bind``` argument. If we do ```--bind ~/savedata_location_of_player_2 ~/savedata```, the program that we will be running using bwrap will think that it's reading and writing to ```~/savedata```, however, the operations are actually being redirected to ```~/savedata_location_of_player_2```.

This means the game won't end up overwriting player 1's savedata, instead, it will have its own save data location. Pretty cool, right?

We actually don't need bubblewrap for Proton games, because the save data is located at ```~/.local/share/Steam/steamapps/compatdata/game_id```

## Why gamescope
When I first started developing this program, I was using [Xephyr](https://wiki.archlinux.org/title/Xephyr), and it was fine, until I realized that Xephyr doesn't have 3D acceleration. While this might be okay for some applications, this meant that some programs that should have worked perfectly fine normally, would have suffered from terrible performance.
To mitigate this issue, I decided to use gamescope, which has 3D acceleration.

## Why are we replacing the ```libsteam_api.so``` or ```steam_api64.dll```
Steam has a DRM (Digital Rights Management) called SteamStub. This DRM prevents us from starting the same game simultaneously.

If you are using GOG, then you don't have to do anything, because GOG games don't have DRM.

## Why are we deleting unity.lock
Unity keeps track of if a game is open or not by checking the presence of a file called ```unity.lock```
If you don't want to deal with deleting this file for every client you open, or you are playing a non-Unity game and you don't know where the lock file is, you can clone the game directory instead.
Some games might not check whether the game is already open or not, so you can skip step 7, if that's the case.

## What if I want to run 2 different games
Then you can just pretty much skip most of these steps. Steams already allows you to launch multiple games simultaneously, given that every game is ran once.
Just make sure that you set the launch arguments correctly.

For native games, you can skip steps 2-8
For Proton games, you can skip steps 2-8

## Is there a limit to how much games can be controlled at once?
I don't think so.

# Building
```
git clone https://github.com/Syntrait/splinux
cd splinux
cargo build -r

# OPTIONAL, this is for decreasing the file size
wget https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz
tar xf upx-4.2.4-amd64_linux.tar.xz
upx-4.2.4-amd64_linux/upx target/release/splinux

target/release/splinux
```

# How to run the games
In this guide, I will be going over how to run Unity games on Steam.
Some things you need to know

Unity games store their save data location at ```~/.config/unity3d/studioname```

If you are trying to play a non-Unity game, you need to find where it stores its save file. (for example, Terraria is stored at ~/.local/share/Terraria)

## Native games
1. You can run the first game from Steam, if you want.
2. For the other instances of the same game, locate the ```libsteam_api.so``` file (for Unity games, located in ```game_name_Data/plugins/```)
3. Replace the ```libsteam_api.so``` with [Goldberg](https://mr_goldberg.gitlab.io/goldberg_emulator/). Goldberg has a patched ```libsteam_api.so``` in the ```linux/``` directory in the archive.
4. Create a directory for the second player's save data location ```mkdir ~/.config/unity3d_second_player```
5. Open a terminal, and navigate to ```~/.local/share/Steam/steamapps/common```
6. Enter the game's directory
7. Delete ```unity.lock``` with ```rm unity.lock```
8. Run the game with
```
gamescope \ # Runs gamescope
-W 1920 \   # Sets width to 1920
-H 540 \    # Sets height to 540
-- \        # Tells gamescope that we're done with the arguments, and the rest is command
bwrap \     # Runs bubblewrap
--dev-bind / / \ # Binds / (root) to itself, so we can access everything under root
--bind ~/.config/unity3d_second_player ~/.config/unity3d \ # Binds the usual save data location for Unity games to its own directory for player 2
./hollow_knight.x86_64 # Runs the game
```

## Proton games
1. You can run the first game from Steam, if you want.
2. For the other instances of the same game, locate the ```steam_api64.dll``` file (for Unity games, located in ```game_name_Data/Plugins/x86_64```
3. Replace the ```steam_api64.dll``` with [Goldberg](https://mr_goldberg.gitlab.io/goldberg_emulator/). Goldberg has a patched ```steam_api64.dll``` in the top level of the archive.
4. Open a terminal, and navigate to ```~/.local/share/Steam/steamapps/common```
5. Enter the game's directory
6. Delete ```unity.lock``` with ```rm unity.lock```
7. Run the game with
```
STEAM_COMPAT_DATA_PATH="$HOME/.local/share/Steam/steamapps/compatdata/game_id_2" \ # Where the save data of the protonfix is located. You can make this wherever you want.
STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.steam/steam" \ # Proton needs the Steam client location.
gamescope \ # Runs gamescope
-W 1920 \   # Sets width to 1920
-H 540 \    # Sets height to 540
-- \        # Tells gamescope that we're done with the arguments, and the rest is command
~/.local/share/Steam/compatibilitytools.d/GE-Proton9-16/proton \ # Proton location
run \ # Tells Proton to run an executable
Lethal\ Company.exe # The executable name
```


# Troubleshooting

## I'm getting a "Permission Denied" error
This error occurs, because Splinux requires raw device access to grab inputs, regardless of the system's state. In order to have raw access to the devices, one of the two conditions must be met

1. The user must be in the "input" group

Add the current user to the input group with

```sudo usermod -aG input $USER```

and then relog to apply the changes.



2. The user trying to get raw device access is the root user.

~~Run the program as root with~~ **This method is currently not working.**

```sudo ./splinux```

## Steam games don't launch when the arguments are set
For reasons I don't know, this works for me when it happens.

1. Go to the game's properties
2. Click on launch options text box
3. Add a space at the end of the line, then delete it.
4. Lose focus from the text box, by clicking outside the box.
5. Launch the game
