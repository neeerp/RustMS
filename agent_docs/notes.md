# Notes (memos)

Notes are short messages left for a character who is offline. The receiver
reads them all at once the next time they play, and reading them deletes
them. The reverse-engineering notes for the v83 client are in report 204m.

## Server

- Storage: the `notes` table holds each waiting note (receiver, sender name,
  text, time sent, flag). `gift_replies` records the Cash Shop gifts whose
  receivers have already thanked the sender, so each gift is thanked once.
  Queries are in `db/src/note.rs`.
- Packets: the client sends `NOTE_ACTION` (0x83) to ask for the list
  (mode 2), to mark notes read (mode 1), or to thank someone for a gift
  (mode 0). Sending a note goes through `USE_CASH_ITEM` with a Memo
  (509xxxx). The server answers with `MEMO_RESULT` (41):
  - mode 3 is the list;
  - mode 4 means the note was sent;
  - mode 5 means it wasn't, with a reason: the receiver is online, there is
    no such character, or their inbox is full;
  - mode 7 tells the client a note has arrived, and the client then asks for
    the list.
- Runtime: `runtime/src/actor/client/notes.rs`.
  - On entering the game, the client is sent its notes if it has any.
  - Sending checks that the receiver exists, isn't the sender, and is offline
    (asked through `DirectoryMessage::Online`). It also checks that the
    sender still holds the Memo and that the inbox holds fewer than 20 notes.
    It then stores the note and uses up one Memo.
  - A thank-you for a gift is stored with flag 1, and the receiver is told at
    once if they are online.
  - Reading deletes the notes. Each flag-1 note raises the reader's fame by
    one, up to the fame limit.

## Client

- `Util/Memos.h` holds the testable rules: the checks before sending, the
  date format, row heights and scrolling, word wrap, and `Memos::Inbox`.
  `Inbox` holds the list the server last sent and makes sure a note arriving
  while a list is on its way is not lost. Tests are in `tests/memos.cpp`.
- `Net/Handlers/MemoHandlers.cpp` handles `MEMO_RESULT`. A non-empty list
  shows the notifier (`UIMemoNotice`, the "You have a new note" box above the
  status bar). The notifier also comes back on each map change while notes
  are still unread.
- `UIMemo` is the inbox. It opens from the notifier's OK button, or from a
  double-click on the status bar near the note icon (v83's exact hotspot,
  which overlaps the Equip key). Closing it, after a warning, marks every note
  read; there is no keeping some.
- `UIMemoSend` is the Send Note dialog. It opens from:
  - using a Memo in the Cash inventory;
  - the Friend tab's Note button, addressed to the picked buddy;
  - a double-click on an offline buddy (an online buddy gets a whisper
    instead).

## Not implemented

- Wedding invitations (flag 3): the client checks for a free Etc slot for
  each, but the server sends invitations as items (`marriage.md`, "Inviting
  guests") and never a note with that flag.
- The window positions are assumed to be centred; v83's default is unknown.
