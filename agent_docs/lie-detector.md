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
   mesos (`PASS_REWARD`) and type 9. A wrong one fails, with type 7. The
   client doesn't end the test itself, so the actor fails it after 60
   seconds (`TEST_TIME`, `lie_due`). Disconnecting in the middle fails it
   too, before the save.
6. A failed player loses up to 7000 mesos (`FAIL_FINE`). They go to the
   tester with type 11, and the fail is logged as a warning. Either end
   tells the directory, which starts the hour.

While a test runs the server ignores `CHANGE_CHANNEL` and
`ENTER_CASH_SHOP`, which the client blocks too.

## `ANTI_MACRO_RESULT` (`0x2B`)

The body is a type and a code: 1 for the item, 2 for the GM skill. Then
whatever the type carries (`build_anti_macro_result`):

| Type | Body | Who | What the client shows |
|---|---|---|---|
| 0 | | tester | "The user cannot be found." |
| 1 | | tester | "...isn't in the middle of attack." |
| 2 | | tester | "This user has already been tested before." |
| 3 | | tester | "...currently going through the Lie Detector Test." |
| 5 | name | tested | "`name` have used the Lie Detector Test." in chat |
| 6 | u8 first, i32 length, JPEG | tested | the picture; `first` starts the 60 seconds |
| 7 | | tested | failed |
| 9 | | tested | passed, with the 5000 mesos line |
| 11 | | tester | "The user has failed... rewarded 7000 mesos" |

The picture is 220x70, a baseline JPEG, which the client decodes with
IJL. It is drawn in Rust from 5x7 glyphs, scaled, slanted and coloured at
random, over noise lines and dots. The glyphs leave out characters that
look like others (I, O, S, Z, B, Q, and the digits 0-2, 5, 6 and 8).

## Not yet

- The GM skill 9001009 (`0x68`), code 2, and the screenshots its types
  take.
- HeavenClient's side: the name prompt, the dialogs and the lock.
