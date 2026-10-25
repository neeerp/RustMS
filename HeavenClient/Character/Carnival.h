#pragma once

#include "../Net/InPacket.h"
#include "../Template/Point.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The Monster Carnival (CField_MonsterCarnival, b163): the sides, the CP
	// screens and the chat log's lines.
	namespace carnival
	{
		// A side: 0 Maple Red, 1 Maple Blue. Anyone else has none (0xff).
		constexpr int8_t NO_TEAM = -1;

		// The field types that read a team from SPAWN_CHAR: the battle field
		// (10), its revive room (11) and the CPQ2 battle field (15).
		bool reads_team(int32_t fieldtype);
		// The battle fields (10 and 15), whose START carries the CP and
		// whose window is up; the revive room's carries only the side.
		bool battle_field(int32_t fieldtype);

		// The type of the field the server has just moved the player to.
		// SPAWN_CHAR and START come before the fade-out loads the field, so
		// the map's own type is still the old one when they are read.
		// Entering a field forgets the battle.
		void enter_field(int32_t mapid, int32_t fieldtype);
		int32_t field_type();
		// That field's map, whose monsterCarnival lists name the window's rows.
		int32_t field_map();

		// "Maple Red", "Maple Blue", or "" for no side (FUN_00941f10): the
		// tag drawn instead of the guild's.
		std::string team_name(int8_t team);

		// CP to spend and CP earned in all.
		struct Points
		{
			uint16_t cp = 0;
			uint16_t total = 0;
		};

		// OBTAINED_CP (0x122), and PARTY_CP (0x123) after its side.
		Points read_points(InPacket& recv);

		// MONSTER_CARNIVAL_START (0x121): the player's side, their CP, their
		// side's and the other's, then how many of each of the field's mobs
		// are out.
		struct Start
		{
			int8_t team = NO_TEAM;
			Points mine;
			Points ours;
			Points theirs;
			std::vector<uint8_t> summoned;
		};

		Start read_start(InPacket& recv);

		// What the client keeps of the battle. A field change forgets it.
		struct State
		{
			int8_t team = NO_TEAM;
			// On a battle field (10 or 15), where a side's mobs spare it.
			bool battle = false;
			Points mine;
			// By side.
			std::array<Points, 2> sides;
			std::vector<uint8_t> summoned;
		};

		State& state();
		void reset();
		// START on the battle field.
		void start(const Start& start);

		// Whether a mob of `mob_team` is on the player's side, so they can
		// neither hit it nor be hurt by it, and it is drawn half see-through
		// (FUN_00670c06).
		bool ally(int8_t mob_team);

		// The chat log's lines (chat type 7) for SUMMON (0x124): `who` spent
		// CP on `what`, from tab `tab` (0 a mob, 1 a skill, 2 a guardian).
		std::string summon_line(uint8_t tab, const std::string& who, const std::string& what);
		// MESSAGE (0x125): why a spend failed. Codes past 5 say nothing.
		std::optional<std::string> message_line(uint8_t code);
		// DIED (0x126): `name` of `team` died, and their side lost `cp`.
		std::string died_line(int8_t team, const std::string& name, uint8_t cp);
		// LEAVE (0x127, chat type 0xc): type 6 is a leader handed over.
		std::string leave_line(uint8_t type, int8_t team, const std::string& name);
		// RESULT (0x128, chat type 0xc): 8 won, 9 lost, 10 a draw, 11 the
		// other side left. Other codes say nothing.
		std::optional<std::string> result_line(uint8_t code);

		// The CP window (CUIMonsterCarnival, b166): what it shows and how it
		// folds, without its art. Its tabs are 0 mobs, 1 skills, 2 guardians.
		namespace window
		{
			constexpr int16_t WIDTH = 162;
			constexpr int16_t FOLDED_HEIGHT = 128;
			constexpr int16_t ROW_HEIGHT = 13;
			// The list's rows, the F-keys' too: no tab has more on a v83 field.
			constexpr size_t MAX_ROWS = 12;
			constexpr uint8_t TABS = 3;
			// Milliseconds an open window stays open while nothing touches it.
			constexpr int32_t FOLD_DELAY = 8000;

			// The list's top in the window, under the tab strip.
			constexpr int16_t LIST_Y = 128;
			// Its first row, under the header.
			constexpr int16_t ROWS_Y = LIST_Y + 19;

			// The rows a tab of `rows` shows.
			size_t shown(size_t rows);
			// The window's height: 128 folded, 13 * (14 + rows shown) open (FUN_0086e047).
			int16_t height(bool open, size_t rows);
			// Where a row's cost starts: v83 counts 5 px a digit back from x 101.
			int16_t cost_x(int32_t cost);

			// The row (from 0) under `pos`, in the window, of `shown` rows
			// (FUN_0086d318). v83 tests the rows 6 px right of where they're
			// drawn.
			std::optional<size_t> row_at(Point<int16_t> pos, size_t shown);
			// The tab under `pos` in the tab strip (CCtrlTab, FUN_004dd827).
			std::optional<uint8_t> tab_at(Point<int16_t> pos);
			// Whether `pos` is on the lock under `shown` rows.
			bool on_lock(Point<int16_t> pos, size_t shown);

			// The window's own state.
			class State
			{
			public:
				// How many rows each tab has.
				explicit State(const std::array<size_t, TABS>& rows = {});

				uint8_t tab() const;
				// The selected row from 1, or 0 for none.
				size_t selected() const;
				bool open() const;
				// Whether the window stays open (icon2), else it folds by itself.
				bool locked() const;
				// The current tab's rows on show.
				size_t shown() const;
				int16_t height() const;
				// Whether BtDecide is up: a row is selected on an open list.
				bool deciding() const;

				// A tab chosen (FUN_00870566): a new one drops the selection,
				// and either way the window opens.
				void choose_tab(uint8_t tab);
				// The Tab key (FUN_008704e9(1)): the next tab on an open
				// window; a folded one opens on its tab.
				void tab_key();
				// F1 to F12 as 0 to 11 (FUN_0087076c): the current tab's row,
				// selected, and the window opens. Nothing if the tab has no
				// such row.
				std::optional<size_t> hotkey(size_t key);
				// A click on the list: the row from 0, or nothing for a miss,
				// which drops the selection. Whether the selection changed.
				bool click(std::optional<size_t> row);
				void toggle_lock();
				void open_up();
				void fold();
				// Anything that keeps the window open a while longer: a row's
				// description shown, say.
				void touch();
				// `ms` milliseconds on; whether the window folded just now
				// (FUN_0087047b).
				bool pass(int32_t ms);

			private:
				std::array<size_t, TABS> rows;
				uint8_t current = 0;
				size_t chosen = 0;
				bool is_open = false;
				bool is_locked = false;
				int32_t timer = FOLD_DELAY;
			};
		}
	}
}
