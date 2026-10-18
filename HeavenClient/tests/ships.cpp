#include "../Gameplay/MapleMap/ClockRules.h"
#include "../Gameplay/MapleMap/ShipRules.h"
#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool near(double value, double expected)
{
	return std::abs(value - expected) < 0.001;
}

int main()
{
	try
	{
		using namespace ms;
		using ship::Action;

		require(ship::on_move(8, 2, ship::DOCKING) == Action::LEAVE, "a platform's ship leaves");
		require(ship::on_move(12, 6, ship::DOCKING) == Action::ENTER, "the other platform's docks");
		require(ship::on_move(8, 2, ship::BALROG) == Action::NONE, "a deck has no ship to sail off");
		require(ship::on_move(10, 4, ship::BALROG) == Action::APPEAR, "the Balrog comes to a deck");
		require(ship::on_move(10, 5, ship::BALROG) == Action::DISAPPEAR, "and goes");
		require(ship::on_move(10, 4, ship::DOCKING) == Action::NONE, "but not to a platform");
		require(ship::on_move(12, 2, ship::DOCKING) == Action::NONE, "each target takes its own event");

		for (uint8_t state : { 0, 1, 6 })
			require(ship::on_state(state, 0, ship::DOCKING) == Action::ENTER, "between trips the ship docks");

		for (uint8_t state : { 2, 3, 4, 5 })
			require(ship::on_state(state, 0, ship::DOCKING) == Action::LEAVE, "under way it sails off");

		require(ship::on_state(7, 0, ship::DOCKING) == Action::NONE, "a later state does nothing");
		require(ship::on_state(3, 1, ship::BALROG) == Action::APPEAR, "a deck the Balrog is at shows it");
		require(ship::on_state(3, 0, ship::BALROG) == Action::NONE, "only when it is there");
		require(ship::on_state(1, 1, ship::BALROG) == Action::NONE, "and the ship under way");

		require(ship::obj_path("Map/Obj/vehicle.img/ship/ossyria/99") == "Obj/vehicle.img/ship/ossyria/99",
				"a shipObj names its art from Map.nx's root");
		require(ship::obj_path("Map/Obj/vehicle/ship/ossyria/97") == "Obj/vehicle.img/ship/ossyria/97",
				"some leave out the .img");
		require(ship::obj_path("Map/Obj/acc8.img/ariant/ship/9") == "Obj/acc8.img/ariant/ship/9",
				"an .img is kept");
		require(ship::signed_y(65341) == -195 && ship::signed_y(-221) == -221, "y is a signed short");

		// Ellinia's platform: the ship waits off to the right until told.
		ship::Motion docking(ship::DOCKING, 1545, 2100, -195, false, 15);
		require(docking.is_shown() && docking.get_x() == 2100, "a docking ship starts away from the dock");
		auto never = [](int32_t) -> int32_t { throw std::runtime_error("a docking ship doesn't hover"); };
		require(docking.enter(), "it sails in");
		docking.update(7500, never);
		require(near(docking.get_x(), 1822.5) && docking.get_y() == -195, "at an even pace");
		docking.update(7500, never);
		require(docking.get_x() == 1545, "to the dock in tMove");
		require(docking.leave(), "and out again");
		require(docking.get_x() == 1545, "from the dock");
		docking.update(20000, never);
		require(docking.get_x() == 2100, "to where it started");
		require(!docking.appear() && !docking.disappear(), "it never fades");

		// The deck from Ellinia: the Balrog's ship comes from the right.
		ship::Motion balrog(ship::BALROG, 485, 485, -221, false, 2);
		require(!balrog.is_shown(), "the Balrog's ship isn't there until it comes");
		require(!balrog.enter() && !balrog.leave(), "it never docks");
		require(!balrog.disappear(), "nor goes before it came");
		auto unused = [](int32_t) -> int32_t { throw std::runtime_error("nothing to hover"); };
		balrog.update(ship::HOVER_INTERVAL, unused);
		require(balrog.appear(), "it comes");
		require(balrog.is_shown() && balrog.get_opacity() == 0.0, "out of nothing");
		require(balrog.get_x() == 585 && balrog.get_y() == -321, "from above and to the right");
		balrog.update(1000, unused);
		require(near(balrog.get_opacity(), 0.5) && balrog.get_x() == 535 && balrog.get_y() == -271,
				"fading in as it slides alongside");

		// A hover step jumps to the last step's end, then glides on: here
		// 20 px left and 49 px down over 1.9 s.
		int32_t draws[] = { 0, 4, 29, 7, 9 };
		int32_t drawn = 0;
		auto next = [&](int32_t below) {
			int32_t value = draws[drawn++];
			require(value < below, "draws stay below their limit");
			return value;
		};
		balrog.update(200, next);
		require(drawn == 5, "the hover draws five numbers every 1.2 s");
		require(balrog.get_x() == 485 && balrog.get_y() == -221, "from where the last step ended");
		require(near(balrog.get_opacity(), 0.6), "a hover cuts the slide but not the fade");
		balrog.update(950, unused);
		require(balrog.get_x() == 475 && near(balrog.get_y(), -196.5), "half way");

		// Far outside the box, the steps head back whatever is drawn.
		require(ship::hover_sign(434, 435, 535, true) == 1, "too far left goes right");
		require(ship::hover_sign(536, 435, 535, false) == -1, "too far right goes left");
		require(ship::hover_sign(435, 435, 535, false) == 1 && ship::hover_sign(535, 435, 535, true) == -1,
				"on an edge goes as drawn");

		require(balrog.disappear(), "it goes");
		auto middle = [](int32_t below) { return below / 2; };
		balrog.update(1000, middle);
		require(near(balrog.get_opacity(), 0.5) && balrog.is_shown(), "fading out, still hovering");
		balrog.update(1000, middle);
		require(balrog.get_opacity() == 0.0, "until gone");
		require(!balrog.disappear(), "a gone ship can't go");

		using wall_clock::Time;
		require(wall_clock::set(0, 5, 0) == Time{ false, 0, 5, 0 }, "midnight is AM 00");
		require(wall_clock::set(12, 0, 0) == Time{ true, 12, 0, 0 }, "noon is PM 12");
		require(wall_clock::set(13, 5, 9) == Time{ true, 1, 5, 9 }, "the afternoon is PM");
		require(wall_clock::set(23, 59, 59) == Time{ true, 11, 59, 59 }, "until PM 11");

		Time evening = wall_clock::set(23, 59, 59);
		require(wall_clock::after(evening, 1) == Time{ true, 12, 0, 0 }, "PM 11:59 goes on to PM 12");
		require(wall_clock::after(Time{ false, 11, 59, 30 }, 3600 + 45) == Time{ false, 13, 0, 15 },
				"and AM 11 to AM 13, as v83 never flips them");
		require(wall_clock::after(Time{ true, 23, 30, 0 }, 3600) == Time{ true, 0, 30, 0 }, "hours wrap at 24");

		require(wall_clock::colon_opacity(4) == 1.0f && wall_clock::colon_opacity(5) < 0.26f,
				"the colon blinks");

		wall_clock::Layout layout = wall_clock::layout(200, 200, 26, 35, 17, 22);
		require(layout.tag == 16 && layout.y == 82, "the glyphs are centred in the box");
		require(layout.hour_tens == 55 && layout.hour_ones == 83 && layout.colon == 111, "the hour and colon");
		require(layout.minute_tens == 130 && layout.minute_ones == 158, "the minutes");

		wall_clock::Clock clock;
		clock.update(5000);
		require(clock.get() == Time{}, "unset, the clock stands at AM 00:00");
		clock.set(wall_clock::set(7, 14, 9));
		require(clock.get() == Time{}, "a setting shows nothing at once");
		clock.update(992);
		require(clock.get() == Time{}, "nor for a second");
		clock.update(8);
		require(clock.get() == Time{ false, 7, 14, 10 }, "then it catches up");
		clock.update(1500);
		require(clock.get() == Time{ false, 7, 14, 11 }, "a second or more at a time");
		clock.update(500);
		require(clock.get() == Time{ false, 7, 14, 11 }, "since the last refresh");
		clock.update(500);
		require(clock.get() == Time{ false, 7, 14, 12 }, "counting from the setting");

		std::cout << "Ship rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
