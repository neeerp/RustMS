#include "../Character/Carnival.h"

#include <cstring>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition)
			throw std::runtime_error(message);
	}

	struct Writer
	{
		std::vector<int8_t> bytes;

		template <typename T>
		Writer& put(T value)
		{
			int8_t raw[sizeof(T)];
			std::memcpy(raw, &value, sizeof(T));
			bytes.insert(bytes.end(), raw, raw + sizeof(T));
			return *this;
		}

		ms::InPacket packet() const
		{
			return ms::InPacket(bytes.data(), bytes.size());
		}
	};
}

int main()
{
	try
	{
		using namespace ms;

		require(carnival::team_name(0) == "Maple Red", "side 0 is Maple Red");
		require(carnival::team_name(1) == "Maple Blue", "side 1 is Maple Blue");
		require(carnival::team_name(carnival::NO_TEAM).empty(), "no side, no tag");
		require(carnival::team_name(2).empty(), "only two sides");

		require(carnival::reads_team(10) && carnival::reads_team(11) && carnival::reads_team(15),
			"the battle fields and the revive room read a team");
		require(!carnival::reads_team(0) && !carnival::reads_team(16), "other fields don't");
		require(carnival::battle_field(10) && carnival::battle_field(15) && !carnival::battle_field(11),
			"the battle fields, not the revive room");

		// START: Blue, 5/9 of their own, 12/20 for Blue, 3/4 for Red, and
		// three mobs on the field's list, the second out twice.
		Writer start;
		start.put<int8_t>(1).put<int16_t>(5).put<int16_t>(9).put<int16_t>(12).put<int16_t>(20);
		start.put<int16_t>(3).put<int16_t>(4).put<uint8_t>(0).put<uint8_t>(2).put<uint8_t>(0);
		InPacket recv = start.packet();
		carnival::Start read = carnival::read_start(recv);
		require(!recv.available(), "the whole of START is read");
		require(read.team == 1, "the side");
		require(read.mine.cp == 5 && read.mine.total == 9, "the player's CP");
		require(read.ours.cp == 12 && read.ours.total == 20, "their side's");
		require(read.theirs.cp == 3 && read.theirs.total == 4, "the other side's");
		require(read.summoned == std::vector<uint8_t>{ 0, 2, 0 }, "the mobs out");

		// CP past 32767 is still CP.
		Writer cp;
		cp.put<uint16_t>(40000).put<uint16_t>(65535);
		InPacket cprecv = cp.packet();
		carnival::Points points = carnival::read_points(cprecv);
		require(points.cp == 40000 && points.total == 65535 && !cprecv.available(), "CP is unsigned");

		// Outside a battle no mob is on the player's side.
		carnival::reset();
		require(!carnival::ally(0) && !carnival::ally(carnival::NO_TEAM), "no sides before START");

		carnival::start(read);
		require(carnival::state().team == 1 && carnival::state().battle, "START puts the player on a side");
		require(carnival::state().sides[1].total == 20 && carnival::state().sides[0].total == 4,
			"the sides' CP by side");
		require(carnival::ally(1), "Blue's mobs spare Blue");
		require(!carnival::ally(0), "Red's don't");
		require(!carnival::ally(carnival::NO_TEAM), "nor the field's mobs of no side");

		carnival::enter_field(11);
		require(carnival::field_type() == 11, "the field being entered");
		require(carnival::state().team == carnival::NO_TEAM && !carnival::ally(1), "a new field forgets the battle");

		// The revive room's START is the side alone; no mob there is an ally.
		carnival::state().team = 1;
		require(!carnival::state().battle && !carnival::ally(1), "the revive room has no battle");

		carnival::enter_field(0);
		require(carnival::field_type() == 0 && carnival::state().team == carnival::NO_TEAM, "leaving forgets the side");

		require(carnival::summon_line(0, "Ann", "Brown Teddy") == "[Ann] has summoned a being. [Brown Teddy]",
			"a mob");
		require(carnival::summon_line(1, "Ann", "Darkness") == "[Ann] has used a skill. [Darkness]", "a skill");
		require(carnival::summon_line(2, "Ann", "Power Up") == "[Ann] has summoned the being. [Power Up]",
			"a guardian");

		require(carnival::message_line(1) == std::optional<std::string>("You don't have enough CP to continue."),
			"not enough CP");
		require(carnival::message_line(4) == std::optional<std::string>("This being is already summoned."),
			"a guardian out");
		require(!carnival::message_line(0) && !carnival::message_line(6), "other codes say nothing");

		require(carnival::died_line(0, "Ann", 10) ==
			"[Ann] has become unable to fight and [Maple Red]team has lost 10 CP.", "a death's CP");
		require(carnival::died_line(5, "Bo", 0) ==
			"[Bo] has become unable to fight but [Maple Blue] has no CP so [Maple Blue] team did not lose any CP",
			"a death with no CP, anything but Red is Blue");

		require(carnival::leave_line(0, 1, "Bo") == "[Maple Blue] of Team [Bo] has quit the Monster Carnival.",
			"v83's order");
		require(carnival::leave_line(6, 0, "Cy") ==
			"Since the leader of the Team [Maple Red] quit the Monster Carnival, [Cy] has been appointed as the new "
			"leader of the team.", "a new leader");

		require(carnival::result_line(8) == std::optional<std::string>(
			"You have won the Monster Carnival. Please wait as you'll be transported out of here shortly."), "a win");
		require(carnival::result_line(10)->rfind("Despite the Overtime", 0) == 0, "a draw");
		require(carnival::result_line(11)->find("opposing team") != std::string::npos, "the other side left");
		require(!carnival::result_line(7) && !carnival::result_line(12), "other codes say nothing");
	}
	catch (const std::exception& e)
	{
		std::cerr << "carnival: " << e.what() << '\n';
		return 1;
	}

	std::cout << "carnival: ok\n";
	return 0;
}
