#pragma once

#include "../Net/InPacket.h"

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
		void enter_field(int32_t fieldtype);
		int32_t field_type();

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
	}
}
