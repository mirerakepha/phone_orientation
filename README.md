# Phone Orientation Viewer

Watch your phone's orientation and movement in 3D, live, on your desktop. A Rust app draws a phone-shaped cuboid, a floor grid and RGB axes using a pure **software renderer** (no GPU), while an Android app streams sensor data over **USB + adb**.

<p align="center">
  <img width="48%" alt="Viewer screenshot 1" src="https://github.com/user-attachments/assets/e1c0643a-9a4d-4fbe-8bb4-4c94855a8f5b" />
  <img width="48%" alt="Viewer screenshot 2" src="https://github.com/user-attachments/assets/19a9bbd4-4446-4ee5-ade5-381b8eee293f" />
</p>
<p align="center">
  <img width="48%" alt="Viewer screenshot 3" src="https://github.com/user-attachments/assets/ef724c03-0a72-4c1b-ae17-bf5271b5bb53" />
  <img width="48%" alt="Viewer screenshot 4" src="https://github.com/user-attachments/assets/86c88256-c3df-4c70-a30f-74532fbd6aab" />
</p>

## Features

- Software-rendered 3D: perspective projection, Bresenham lines, filled triangles, back-face culling and simple lighting, all on the CPU
- Quaternion-based rotation, with smoothing so sensor jitter looks steady
- Live position and orientation from a real Android device
- Floor grid and RGB axes
- Keyboard control for testing without a phone

## How it works

```
Phone sensors -> Android app (Kotlin) -> TCP 127.0.0.1:8765 -> adb reverse (USB) -> viewer (minifb)
```

The Android app reads the rotation and linear-acceleration sensors and sends one line per update. `adb reverse` tunnels the phone's localhost port to the PC over USB, so no Wi-Fi or IP address is needed. The viewer listens on that port, parses each line into a pose and draws it.

## Requirements

- Rust and Cargo
- Android Studio and an Android phone with USB debugging enabled
- `adb` available on your PATH

## Run it

**1. Start the viewer**

```bash
cargo run
```

**2. Connect the phone** (USB debugging on), then create the tunnel:

```bash
adb devices
adb reverse tcp:8765 tcp:8765
```

If more than one device is listed, target your phone explicitly:

```bash
adb -s <your-device-serial> reverse tcp:8765 tcp:8765
```

The tunnel is lost when the phone is unplugged or adb restarts, so repeat this command when that happens.

**3. Launch the Android app** on the phone. The cuboid starts following it. Tap the phone's screen to re-zero position.

To use a different port, change it in three places: the Rust listener, the Android socket, and the `adb reverse` command.

## Controls (keyboard testing)

| Key | Action |
| --- | --- |
| `W` / `S` | Pitch |
| `A` / `D` | Yaw |
| `Q` / `E` | Roll |
| Arrow keys | Move left/right and away/toward the camera |
| `Space` / `Left Shift` | Move up / down |
| `R` | Reset |
| `Esc` | Quit |

While a phone is connected, its data overrides the keyboard.

## Data format

One line of text per update:

```
qw qx qy qz px py pz
```

- `qw qx qy qz`: orientation quaternion in Android's world frame
- `px py pz`: position in meters in Android's world frame (x east, y north, z up)

The viewer converts to its own axes (Y up) and scales position so 1 unit is about 10 cm. You can test without a phone by sending lines yourself:

```bash
nc 127.0.0.1 8765
```

## Limitations

- **Position drifts.** Phones have no direct position sensor, so movement is estimated by integrating the accelerometer. It works for short, smooth motions, and the app limits drift with a dead zone, stillness detection, damping and a ±0.5 m clamp. Tap the screen to re-zero. For accurate free movement, camera-based tracking such as ARCore would be needed.
- **Heading is arbitrary at startup.** The game rotation vector sensor has no compass reference, so "forward" on the phone may not match "away from camera" on screen.

## Troubleshooting

| Problem | Fix |
| --- | --- |
| Nothing moves | Run `adb reverse` again, then check `adb reverse --list` |
| `more than one device/emulator` | Use `adb -s <serial> reverse ...` |
| `Connection refused` in logcat | The tunnel is missing, or the viewer isn't running |
| `could not bind port` | Another program is using the port; pick a different one |

See app logs with `adb logcat -s viewer`.
