# Native client testing

## Fast client logic checks

From the repository root:

```sh
cmake -S HeavenClient -B HeavenClient/cmake-build -DHEAVENCLIENT_BUILD_TESTS=ON
cmake --build HeavenClient/cmake-build --target quest_marker_tests input_focus_tests movement_physics_tests frame_assembler_tests -j4
ctest --test-dir HeavenClient/cmake-build --output-on-failure
```

These tests need no display, OpenGL, database, or NX assets. The CMake test option
defaults to OFF, so ordinary client builds keep their existing behavior.

The checks cover quest marker selection, keyboard routing when a focused panel
closes, and jump momentum/contact transitions in both directions. The physics
test creates a tiny temporary NX foothold fixture and uses the production
collision code. It also checks that releasing an arrow in midair preserves
speed and that takeoff and landing update the grounded flag immediately.

`frame_assembler_tests` covers the network receive loop (`Net/FrameAssembler.h`,
used by `Session`): every split point of a multi-frame stream, a partial
trailing frame, stopping after a reconnect, and impossible lengths. It encrypts
frames with a real `Cryptography` pair, so a dropped or repeated frame shows up
as misdecrypted data.

`map_tooltip_tests` covers the World Map and minimap tooltip's layout
(`IO/Components/MapTooltipLayout.h`): a map's size doesn't depend on the map
shown before it, the padding is added once, the description sits under the
name, and descriptions wrap at the name's width when that is wider.

`answer_focus_tests` covers Left, Right, and Enter on an NPC dialog's YES/NO
and ACCEPT/DECLINE pages (`IO/Components/AnswerFocus.h`): Enter presses YES or
ACCEPT when no answer is focused, and otherwise the one Left or Right focused.

`slider_steps_tests` covers where a System Option slider's thumb stops
(`IO/Components/SliderSteps.h`): the steps spread evenly from end to end, a
thumb dragged between steps snaps to the closer one, and one dragged past
either end stays at that end.

`menu_cover_tests` covers the status bar's open game or shortcut menu
(`IO/Components/MenuCover.h`): inside the menu only its own buttons take the
cursor, so QUIT GAME wins over the Stat icon drawn under it and the menu's frame
presses nothing.

`npc_reach_tests` covers which NPC the Interact key talks to
(`Gameplay/MapleMap/NpcReach.h`): NPCs within 150 px either side and 50 px up or
down are in reach, and the horizontally nearest wins.

`enter_number_tests` covers what a number entry box accepts
(`IO/UITypes/EnterNumber.h`): digits from 1 to the box's maximum, with empty,
zero, too-large (however many digits), and non-digit input refused.
`char_click_tests` covers which clicks land on a character for the character
info window (`Gameplay/MapleMap/CharClick.h`): a box 18 px either side of the
feet and 70 px tall above them.

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
files, fonts, Icon.png, and supported_quests.txt there, and copy Settings. This avoids asset copies
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
