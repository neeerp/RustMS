# The Lie Detector

The Lie Detector (2190000, USE class 219) tests whether a player is
botting. The tester uses it on someone on their map who is fighting, and
the server shows that player a picture of four to six letters and digits
to type back within a minute. Report c025 in v83ref has the client's
side. Cosmic has none of it, so the rules here are ours: the A picks of
lanes/QUESTIONS.md.

## The flow

1. The client asks the tester for a name. It refuses in a town or on a map
   with field limit bit `0x10000` (`FIELD_LIMIT_LIE_DETECTOR`,
   `FieldTemplate::may_lie_detect`), and finds only characters on its own
   map. It sends `USE_LIE_DETECTOR` (`0x67`): the name, the USE slot and
   the item id. It keeps the item for the server to take.
2. `world::UseLieDetectorHandler` checks the item, the map and that the
   name isn't the tester's own, then hands `WorldAction::LieDetect` to the
   client actor. The actor adds its field and asks the directory.
3. `WorldDirectory::lie_detect` finds the name on the tester's channel.
   Someone not found, in the Cash Shop or a hidden GM is "cannot be found".
   Someone whose last test ended less than an hour ago (`TESTED_FOR`) is
   "tested before". Anyone else gets `ServerMessage::LieDetect`.
4. The tested player's actor (`ClientActor::lie_detect`) refuses unless
   they stand on the tester's field, aren't in a test already, and sent an
   attack (close, ranged, magic, touch or summon) in the last 10 seconds
   (`ATTACK_WINDOW`). Otherwise it draws a picture (`net::captcha`) and
   sends them `ANTI_MACRO_RESULT` type 5 with the tester's name, then type
   6 with the JPEG. The tester's actor then takes the item.
5. The client closes the picture when it sends `ANSWER_LIE_DETECTOR`
   (`0x69`) with the text, so every answer is judged at once. The case and
   spaces around the answer don't count. A right answer passes, with 5000
   mesos (`PASS_REWARD`) and type 9. A test takes two answers (`TRIES`),
   as the client's "tries left" counts: a first wrong one gets a new
   picture, type 6 with `first` 0, in the time left, and a second fails,
   with type 7. The client doesn't end the test itself, so the actor fails it after 60
   seconds (`TEST_TIME`, `lie_due`). Disconnecting in the middle fails it
   too, before the save.
6. A failed player loses up to 7000 mesos (`FAIL_FINE`). They go to the
   tester with type 11, and the fail is logged as a warning. Either end
   tells the directory, which starts the hour.

While a test runs the server ignores `CHANGE_CHANNEL` and
`ENTER_CASH_SHOP`, which the client blocks too.

## The GM's skill

A GM's Lie Detector skill (9001009, `LIE_DETECTOR_SKILL`) gives the same
test with code 2. No v83 Skill.nx has the skill, though the client handles
it, so a GM has it only through the `skills` table.

1. The client asks for a name as for the item, but anywhere, and sends
   `USE_LIE_DETECTOR_SKILL` (`0x68`) with only the name.
2. `world::UseLieDetectorSkillHandler` checks for a GM with the skill in
   their book, and the test goes on as `LieDetectorBy::Skill`. The hour
   since the last test doesn't bind it; the field, a test running and the
   attack window do.
3. The tested player isn't told who tests them: only type 6, code 2. The
   GM gets type 5, code 2, with the tested player's name.
4. Nothing is won or lost. A pass is type 9 code 2 ("Thank you for your
   cooperation."), and the GM gets type 10. A fail is type 7 code 2, which
   adds the sanction line, and is logged as a warning. The GM gets type 8
   (`ServerMessage::LieDetectorJudged`).

## `ANTI_MACRO_RESULT` (`0x2B`)

The body is a type and a code: 1 for the item, 2 for the GM skill. Then
whatever the type carries (`build_anti_macro_result`):

| Type | Body | Who | What the client shows |
|---|---|---|---|
| 0 | | tester | "The user cannot be found." |
| 1 | | tester | "...isn't in the middle of attack." |
| 2 | | tester | "This user has already been tested before." |
| 3 | | tester | "...currently going through the Lie Detector Test." |
| 4 | name | | "`name`_The screenshot has been saved. You have been notified of macro-assisted program monitoring." in chat, and a screenshot; never sent |
| 5 | name | tested, code 1 | "`name` have used the Lie Detector Test." in chat |
| 5 | name | GM, code 2 | "`name`_The screenshot has been saved. The Lie Detector has been activated." in chat, and a screenshot |
| 6 | u8 first, i32 length, JPEG | tested | the picture; `first` starts the 60 seconds |
| 7 | | tested | failed; code 2 adds "You will be sanctioned..." |
| 8 | name | GM, code 2 | "`name`_The screenshot has been saved. It appears that you may be using a macro-assisted program." in chat, and a screenshot |
| 9 | | tested | passed, with the 5000 mesos line; code 2 "Thank you for your cooperation." |
| 10 | name | GM, code 2 | "`name`_You have passed the Lie Detector Test." in chat |
| 11 | | tester | "The user has failed... rewarded 7000 mesos" |

The `name`_ in the GM's lines is the start of the screenshot's file name,
`name`_YYYYMMDD_HHMMSS.jpg. Who gets types 5 code 2, 8 and 10 is ours: the
client can't tell, and they name the player tested.

The picture is 192x42, the size of the box in the challenge's art, as a
baseline JPEG, which the client decodes with
IJL. It is drawn in Rust from 5x7 glyphs, scaled, slanted and coloured at
random, over noise lines and dots. The glyphs leave out characters that
look like others (I, O, S, Z, B, Q, U, and the digits 0-2, 5, 6 and 8).

## HeavenClient

- Double-clicking a 219xxxx item (`UIItemInventory`, `use_lie_detector`)
  checks the request gate, an open mini room, a ride, a town, and the
  field limit `LIE_DETECTOR` (0x10000). It then opens
  `UILieDetectorName` on `Macro/backgrnd`. The typed name must match
  someone on the map exactly, or the client shows type 0 itself without
  sending anything. Otherwise it sends `USE_LIE_DETECTOR`.
- Skill 9001009 (`Combat::request_lie_detector`) checks an open mini room
  and a ride, then asks for the name the same way and sends
  `USE_LIE_DETECTOR_SKILL`, or shows type 0 code 2.
- `AntiMacroResultHandler` (`InventoryHandlers`) puts types 4, 5, 8 and 10
  in chat (`lie_detector::line`) and takes the screenshots that go with
  them (`Window::screenshot(name)`). For type 6 it decodes the JPEG
  (`Graphics/Jpeg`, stb_image) into a `PixelTexture` and opens
  `UILieDetector` at the top right, with the `num1` clock and the tries
  left. The item's test is on `Macro/backgrnd4`. The skill's is on
  `MacroSkill/backgrnd2`, laid out the same, though UIClassic's has its
  words in Korean. Types 7 and 9
  close it, and every type with a text opens `UILieDetectorResult` on
  `MacroSkill/backgrnd3` with a `Macro/admin` face.
- `lie_detector::time_left` (`Gameplay/LieDetector.h`) is the test's
  clock. A first picture sets it to 60 seconds, an answer keeps what is
  left for the next picture, and a result or a new field clears it.
  While it runs, the Cash Shop and the status bar's channel button say
  "You cannot use it in the middle of the Lie Detector Test.", and an
  Aran's combo stays frozen. The counter stops being sent, the combo
  skills are refused, the combo doesn't time out, and a thaw restarts the
  clock.
- The layout follows UIClassic's art rather than the 2025 client's
  positions, which suit other art. UIClassic's art has no "tries left"
  label, so the client draws it as text.

## Not yet

- The client's own check that the person named is fighting (it shows
  type 1 code 1); the server's check answers instead.
- The 2025 client also keeps the combo HUD from fading while frozen.
