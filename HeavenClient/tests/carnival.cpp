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

		carnival::enter_field(980000102, 11);
		require(carnival::field_type() == 11 && carnival::field_map() == 980000102, "the field being entered");
		require(carnival::state().team == carnival::NO_TEAM && !carnival::ally(1), "a new field forgets the battle");

		// The revive room's START is the side alone; no mob there is an ally.
		carnival::state().team = 1;
		require(!carnival::state().battle && !carnival::ally(1), "the revive room has no battle");

		carnival::enter_field(100000000, 0);
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

		// The CP window's geometry (b166).
		{
			using namespace carnival::window;

			require(height(false, 10) == 128 && height(true, 10) == 13 * 24 && height(true, 0) == 13 * 14,
				"folded 128, open 13 * (14 + rows)");
			require(height(true, 20) == 13 * 26 && shown(20) == 12, "at most 12 rows");
			require(cost_x(7) == 96 && cost_x(17) == 91 && cost_x(150) == 86, "costs end at 101, 5 px a digit");

			require(row_at(Point<int16_t>(6, 147), 3) == std::optional<size_t>(0), "the first row's corner");
			require(row_at(Point<int16_t>(143, 159), 3) == std::optional<size_t>(0), "its far corner");
			require(row_at(Point<int16_t>(50, 160), 3) == std::optional<size_t>(1), "the second row");
			require(!row_at(Point<int16_t>(5, 150), 3) && !row_at(Point<int16_t>(144, 150), 3),
				"6 px right of where they're drawn");
			require(!row_at(Point<int16_t>(50, 146), 3) && !row_at(Point<int16_t>(50, 147 + 39), 3),
				"nothing above the rows or past them");

			require(tab_at(Point<int16_t>(5, 111)) == std::optional<uint8_t>(0), "the first tab");
			require(tab_at(Point<int16_t>(30, 120)) == std::optional<uint8_t>(1), "the second");
			require(tab_at(Point<int16_t>(112, 127)) == std::optional<uint8_t>(2), "the third, 2 px past its end");
			require(!tab_at(Point<int16_t>(28, 120)) && !tab_at(Point<int16_t>(30, 110)) &&
				!tab_at(Point<int16_t>(30, 128)), "between tabs, above the click line, under the strip");

			require(on_lock(Point<int16_t>(9, 13 * 3 + 162), 3) && on_lock(Point<int16_t>(17, 13 * 3 + 173), 3),
				"the lock under three rows");
			require(!on_lock(Point<int16_t>(18, 13 * 3 + 162), 3) && !on_lock(Point<int16_t>(9, 162), 3),
				"and nowhere else");

			State window({ 10, 8, 9 });
			require(!window.open() && window.tab() == 0 && window.height() == 128, "it starts folded on mobs");
			require(!window.deciding(), "nothing to decide");

			// The Tab key opens a folded window on its tab, then goes round.
			window.tab_key();
			require(window.open() && window.tab() == 0 && window.height() == 13 * 24, "Tab opens it");
			window.tab_key();
			window.tab_key();
			require(window.tab() == 2 && window.shown() == 9, "and then cycles");
			window.tab_key();
			require(window.tab() == 0, "back to mobs");

			require(window.click(2) && window.selected() == 3 && window.deciding(), "a click picks a row");
			require(!window.click(2), "the same row again changes nothing");
			window.choose_tab(0);
			require(window.selected() == 3, "the same tab keeps it");
			window.choose_tab(1);
			require(window.selected() == 0 && window.tab() == 1, "another tab drops it");
			window.click(1);
			require(window.click(std::nullopt) && window.selected() == 0, "a miss drops it");

			// F-keys pick the current tab's rows and open the window.
			window.fold();
			require(window.hotkey(7) == std::optional<size_t>(7) && window.selected() == 8 && window.open(),
				"F8 on skills");
			require(!window.hotkey(8) && window.selected() == 8, "skills have no ninth");
			require(!window.hotkey(12), "only F1 to F12");
			window.fold();
			require(!window.hotkey(9) && !window.open() && window.selected() == 8,
				"a key with no row leaves the window be");
			window.choose_tab(2);
			window.fold();
			window.tab_key();
			require(window.open() && window.tab() == 2, "Tab opens a folded window on its own tab");
			require(!window.click(std::nullopt), "a miss with nothing selected changes nothing");
			window.fold();
			window.click(0);
			require(!window.deciding(), "nothing to decide while folded");
			window.open_up();
			State empty({ 0, 0, 0 });
			require(!empty.hotkey(0) && !empty.deciding(), "an empty tab has no rows to pick");
			require(cost_x(0) == 96 && cost_x(1000) == 81, "one digit, four digits");
			require(row_at(Point<int16_t>(50, 185), 3) == std::optional<size_t>(2), "the last row's bottom");

			// Left alone, an open window folds after 8 seconds unless locked.
			require(!window.pass(8000), "not yet");
			require(window.pass(1) && !window.open(), "folded");
			require(!window.pass(10000), "a folded window stays so");
			window.open_up();
			window.toggle_lock();
			require(window.locked() && !window.pass(10000) && window.open(), "the lock keeps it open");
			window.toggle_lock();
			window.pass(7000);
			window.touch();
			require(!window.pass(7000) && window.open(), "a touch puts the fold off");

			// The side panel's counts: START's mobs out, then each summon.
			Tally tally;
			tally.add("Brown Teddy", 2);
			require(tally.entries().size() == 1 && tally.entries()[0].count == 2 && tally.latest().empty(),
				"START's mobs are rows, none the latest");
			require(tally.summon("Brown Teddy") == 0 && tally.entries()[0].count == 3 && tally.latest() == "Brown Teddy",
				"a summon counts its row up");
			require(tally.summon("Bloctopus") == 1 && tally.entries()[1].count == 1 && tally.latest() == "Bloctopus",
				"a new name, a new row at the end");
			require(tally.range() == 0, "no scrolling for 5 rows or fewer");
			tally.scroll(3);
			require(tally.top() == 0, "nor any scrolling");

			for (const char* name : { "A", "B", "C", "D" })
				tally.summon(name);
			require(tally.entries().size() == 6 && tally.range() == 2, "6 rows scroll to the count less 4");
			tally.scroll(tally.summon("E"));
			require(tally.top() == 3, "a new row scrolls to the end");
			tally.scroll(tally.summon("Bloctopus"));
			require(tally.top() == 1 && tally.entries()[1].count == 2, "an old one as far as its row");
			tally.scroll(0);
			require(tally.top() == 0, "back to the top");
			tally.scroll(100);
			require(tally.top() == tally.range(), "no further than the range");
			tally.add("Brown Teddy", 1);
			require(tally.latest() == "Bloctopus" && tally.top() == 3 && tally.entries().size() == 8,
				"an added row leaves the latest and the scroll be");
			require(tally.summon("Brown Teddy") == 0 && tally.entries()[0].count == 4 && tally.entries()[7].count == 1,
				"a summon counts the first row of its name");

			Tally few;
			require(few.range() == 0 && few.entries().empty(), "an empty panel");
			for (const char* name : { "A", "B", "C", "D", "E" })
				few.summon(name);
			require(few.range() == 0, "5 rows don't scroll");
		}
	}
	catch (const std::exception& e)
	{
		std::cerr << "carnival: " << e.what() << '\n';
		return 1;
	}

	std::cout << "carnival: ok\n";
	return 0;
}
