# Native client testing

## Fast client logic checks

From the repository root:

```sh
cmake -S HeavenClient -B HeavenClient/cmake-build -DHEAVENCLIENT_BUILD_TESTS=ON
cmake --build HeavenClient/cmake-build --target quest_marker_tests -j4
ctest --test-dir HeavenClient/cmake-build --output-on-failure
```

These tests need no display, OpenGL, database, or NX assets. The CMake test option
defaults to OFF, so ordinary client builds keep their existing behavior.

## GUI checks without taking desktop focus

Prefer a private Xvfb display for automated native-client checks. It gives the
client its own mouse, keyboard focus, and screen; screenshots and recordings can
be captured there without moving the user's desktop pointer or activating windows.

The client successfully runs with software OpenGL on this machine. Set these
variables only for the test client's process:

- `DISPLAY`: the private Xvfb display
- `LIBGL_ALWAYS_SOFTWARE=1`
- `__GLX_VENDOR_LIBRARY_NAME=mesa`
- `ALSOFT_DRIVERS=null` for silent GUI testing

Use a separate ignored working directory for each test client. Symlink the NX
files, fonts, and Icon.png there, and copy Settings. This avoids asset copies
and keeps GUI test preferences separate from the desktop client's settings.
Launch the compiled client by its absolute path from that directory.

Use `Xvfb -displayfd <fd> -screen 0 1280x800x24 -nolisten tcp` to allocate an
unused display, or use `xvfb-run -a`. Record the display number and process IDs.
Every automation command must target that display explicitly, for example:

```sh
DISPLAY="$test_display" xdotool search --name '^MapleStory$'
DISPLAY="$test_display" xdotool windowfocus --sync "$test_window"
DISPLAY="$test_display" xdotool mousemove --window "$test_window" 264 333 click 1
import -display "$test_display" -window "$test_window" screenshot.png
```

Use `windowfocus` inside Xvfb; `windowactivate` expects a window manager. FFmpeg's
x11grab can record the private display/window. Stop the test client and Xvfb
processes when verification is complete. For full backend isolation, connect
the GUI test client to the integration harness using ServerPort 18484 instead
of the ordinary development login port 8484.
