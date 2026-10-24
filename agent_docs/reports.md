# Reporting a player

v83 lets a player report another to a GM ("claim" in the client's
names): for what they said, or for using an illegal program. The
reporter's client gathers the report, the server keeps it in the `reports`
table, and the reporter is told whether it was kept and how many reports
they have left this week. Nothing yet shows the reports to a GM; read the
table.

## The flow

1. On entering the world the server sends `CLAIM_AVAILABLE_TIME` (`0x2E`)
   with hours 0 and 0, which means all day, and `CLAIM_STATUS_CHANGED`
   (`0x2F`) with 1. Without the second the client answers every report
   with "The reporting system is currently undergoing maintenence check."
   (`ClientActor::open_claims`).
2. The client sends `CLAIM` (`0x6A`): a byte, 1 for a conversation or 0
   for an illegal program; the character's name; the reason; the
   description; and for a conversation the chat lines, at most 1,600
   characters. A program report's reason is always 9. A conversation's is
   1, 2, 3, 5, 7 or 8, after the client's list of reasons.
3. `world::ClaimHandler` reads it. With fewer than 300 mesos
   (`CLAIM_FEE`) it replies "not enough mesos" at once. Otherwise it hands
   `WorldAction::Claim` to the client actor.
4. `ClientActor::claim` checks the mesos again and calls
   `db::report::file`, which works in one transaction. It finds the victim
   by name, ignoring case. If no one has that name, or it is the
   reporter's own, the reply is "re-check the name". If the reporter made
   10 reports in the last seven days (`CLAIMS_A_WEEK`, Cosmic's limit),
   the reply is "exceeded". Otherwise the row is kept. Description and
   chat lines are cut to 2,000 characters.
5. A kept report costs 300 mesos. The player is saved, and the client gets
   the meso stat change and `CLAIM_RESULT` (`0x2D`) mode 2.

## `CLAIM_RESULT` (`0x2D`)

The client shows each mode in an OK box (v83 `FUN_00a27891`):

| Mode | Body | Message |
|---|---|---|
| 2 | byte today, int left | kept; `left` reports left this week, or none left. If `today` is 0: no more today. When a client test (probably whether the player is a GM) holds, the box says only "You have successfully registered." |
| `0x41` | | "Please try again later." (a database error) |
| `0x42` | | "Please re-check the character name" |
| `0x43` | | "You do not have enough mesos to report." |
| `0x45` | | "You have exceeded the number of reports available." |

The server always sends `today` as 1, because there is no daily limit.
The client also knows modes 3, `0x44`, `0x47` and `0x48`; the server
never sends them. Mode 3 tells the reported player they were reported.

## Not modelled

- No report hours: the window is always all day.
- No daily limit, and no message to the reported player.
- No GM view or punishment of the reports.

## Tests

- Packet bytes: `net::packet::build::world::claim`.
- Parsing and the fee: `net::packet::handle::world::claim`.
- The whole flow: `integration-harness/tests/claim.rs`.
- The harness's login reads the two entry packets, the last of entering
  the game, and keeps their bodies in `WorldSession::claims`. A test's
  first read is then whatever comes after entry.
