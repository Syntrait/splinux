# Splinux
A tool for splitting the screen on Linux, and passing inputs to them 

# Usage

## Starting the program
In a 1920x1080 screen, to split the program horizontally, run both programs with

Launch your program (ex. librewolf) with

```gamescope -W 1920 -H 540 -- librewolf```

If the program is a game on Steam, add this to the command line arguments

```gamescope -W 1920 -H 540 -- %command%```

## Identifying the display ids
After launching your programs on seperate gamescope sessions, you need to know their display ids
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
XWayland is a compatibility layer for Wayland. Wayland is still in development, and some programs are yet to adopt it, and still use X11. To run these applications in Wayland, we need a compatibility layer. That's what Xwayland is.
Xwayland will run a X server, and assign a display number to it (ex. DISPLAY=:30)

Every X server can only have **one** focused window, however, we can nest multiple X servers inside each other, and if we open multiple X servers, we can have multiple focused windows, and because of the isolation, they won't be affecting each other.

## Why gamescope
When I first started developing this program, I was using [Xephyr](https://wiki.archlinux.org/title/Xephyr), and it was fine, until I realized that Xephyr doesn't have 3D acceleration. While this might be okay for some applications, this meant that some programs that should have worked perfectly fine normally, would have suffered from terrible performance.
To mitigate this issue, I decided to use gamescope, which has 3D acceleration.

# Building
```
git clone https://github.com/Syntrait/splinux
cd splinux
cargo build -r

# OPTIONAL, this is for decreasing the file size
wget https://github.com/upx/upx/releases/download/v4.2.4/upx-4.2.4-amd64_linux.tar.xz
tar xf upx-4.2.4-amd64_linux.tar.xz
upx-4.2.4-amd64_linux/upx target/release/splinux

Your binary is ready at target/release/splinux
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
