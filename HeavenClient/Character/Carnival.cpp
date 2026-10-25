#include "Carnival.h"

namespace ms
{
	namespace carnival
	{
		namespace
		{
			int32_t entered = 0;

			bool has_side(int8_t team)
			{
				return team == 0 || team == 1;
			}
		}

		bool reads_team(int32_t fieldtype)
		{
			return fieldtype == 10 || fieldtype == 11 || fieldtype == 15;
		}

		bool battle_field(int32_t fieldtype)
		{
			return fieldtype == 10 || fieldtype == 15;
		}

		void enter_field(int32_t fieldtype)
		{
			entered = fieldtype;
			reset();
		}

		int32_t field_type()
		{
			return entered;
		}

		std::string team_name(int8_t team)
		{
			switch (team)
			{
			case 0:
				return "Maple Red";
			case 1:
				return "Maple Blue";
			default:
				return "";
			}
		}

		Points read_points(InPacket& recv)
		{
			Points points;
			points.cp = static_cast<uint16_t>(recv.read_short());
			points.total = static_cast<uint16_t>(recv.read_short());

			return points;
		}

		Start read_start(InPacket& recv)
		{
			Start start;
			start.team = recv.read_byte();
			start.mine = read_points(recv);
			start.ours = read_points(recv);
			start.theirs = read_points(recv);

			// One for each of the field's mob list; the server sends as many.
			while (recv.available())
				start.summoned.push_back(static_cast<uint8_t>(recv.read_byte()));

			return start;
		}

		State& state()
		{
			static State carnival;

			return carnival;
		}

		void reset()
		{
			state() = {};
		}

		void start(const Start& start)
		{
			State& now = state();
			now.team = start.team;
			now.battle = true;
			now.mine = start.mine;
			now.summoned = start.summoned;

			if (has_side(start.team))
			{
				now.sides[start.team] = start.ours;
				now.sides[1 - start.team] = start.theirs;
			}
		}

		bool ally(int8_t mob_team)
		{
			const State& now = state();

			return now.battle && has_side(now.team) && mob_team == now.team;
		}

		std::string summon_line(uint8_t tab, const std::string& who, const std::string& what)
		{
			switch (tab)
			{
			case 0:
				return "[" + who + "] has summoned a being. [" + what + "]";
			case 1:
				return "[" + who + "] has used a skill. [" + what + "]";
			default:
				return "[" + who + "] has summoned the being. [" + what + "]";
			}
		}

		std::optional<std::string> message_line(uint8_t code)
		{
			switch (code)
			{
			case 1:
				return "You don't have enough CP to continue.";
			case 2:
				return "You can no longer summon the Monster.";
			case 3:
				return "You can no longer summon the being.";
			case 4:
				return "This being is already summoned.";
			case 5:
				return "This request has failed due to an unknown error.";
			default:
				return std::nullopt;
			}
		}

		std::string died_line(int8_t team, const std::string& name, uint8_t cp)
		{
			// Anything but Red is Blue here.
			std::string side = team == 0 ? team_name(0) : team_name(1);

			if (cp != 0)
				return "[" + name + "] has become unable to fight and [" + side + "]team has lost " +
					std::to_string(cp) + " CP.";

			return "[" + name + "] has become unable to fight but [" + side + "] has no CP so [" + side +
				"] team did not lose any CP";
		}

		std::string leave_line(uint8_t type, int8_t team, const std::string& name)
		{
			std::string side = team == 0 ? team_name(0) : team_name(1);

			// v83 passes the side first and the name second to both (A
			// 565a42, 565a79), so the first reads "[Maple Red] of Team [name]".
			if (type == 6)
				return "Since the leader of the Team [" + side + "] quit the Monster Carnival, [" + name +
					"] has been appointed as the new leader of the team.";

			return "[" + side + "] of Team [" + name + "] has quit the Monster Carnival.";
		}

		std::optional<std::string> result_line(uint8_t code)
		{
			const std::string wait = " Please wait as you'll be transported out of here shortly.";

			switch (code)
			{
			case 8:
				return "You have won the Monster Carnival." + wait;
			case 9:
				return "Unfortunately, you have lost the Monster Carnival." + wait;
			case 10:
				return "Despite the Overtime, the carnival ended in a draw." + wait;
			case 11:
				return "Monster Carnival has ended abruptly due to the opposing team leaving the game too early." +
					wait;
			default:
				return std::nullopt;
			}
		}
	}
}
