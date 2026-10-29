#include "../Gameplay/Combat/Massacre.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::massacre;

		Session state;
		require(store(state, "massacre_hit", 12) && store(state, "massacre_miss", 3), "the counts are kept");
		require(store(state, "massacre_cool", 4) && store(state, "massacre_skill", 2), "all four");
		require(state.hit == 12 && state.miss == 3 && state.cool == 4 && state.skill == 2, "each in its own");
		require(!store(state, "energy", 5), "but not the Dojo's energy");

		start_clock(state, 0, 120);
		state.count = 30;
		require(state.length == 120 && state.clock.is_shown(), "the clock sets the stage's length");
		enter_field(state, FIELD_RESULT);
		require(state.field == FIELD_RESULT && state.hit == 12 && state.skill == 2, "the counts outlast the field");
		require(state.count == 0 && state.length == 0 && !state.clock.is_shown(), "the gauge and clock don't");

		require(rage_ready(1) && !rage_ready(0), "Rage of Pharaoh takes a use left");

		require(gauge(120, 120, 1, 0, 100) == 0, "the gauge starts empty");
		require(gauge(120, 90, 1, 0, 100) == 30, "and rises by the decrease each second");
		require(gauge(120, 90, 2, 0, 100) == 60, "at the map's rate");
		require(gauge(120, 90, 1, 10, 100) == 20, "the server's count pulls it back");
		require(gauge(120, 110, 1, 50, 100) == 0, "never below empty");
		require(gauge(120, 0, 1, 0, 100) == 100, "nor above full");

		require(gauge_width(0, 100) == 0 && gauge_width(100, 100) == GAUGE_WIDTH, "the bar is 259 pixels full");
		require(gauge_width(50, 100) == 129 && gauge_width(5, 0) == 0, "rounded down, none with no total");
		require(!danger(64, 100) && danger(65, 100) && !danger(5, 0), "red from 65%");

		using Digits = std::vector<std::pair<int32_t, int32_t>>;
		require(digits(0, 5) == Digits{ { 4, 0 } }, "0 is one 0 in the last slot");
		require(digits(305, 5) == Digits{ { 4, 5 }, { 3, 0 }, { 2, 3 } }, "right aligned, no leading zeros");
		require(digits(123456, 5).size() == 5, "no more than the slots");
		require(digits(7, 1) == Digits{ { 0, 7 } }, "the skill's single slot");

		KeyPanel key;
		require(key.get() == KeyPanel::State::HIDDEN, "the key starts hidden");
		key.skill_changed(0, 1);
		require(key.get() == KeyPanel::State::OPENING, "and opens with the first use");
		key.finished();
		require(key.get() == KeyPanel::State::SHOWN, "then stays");
		key.skill_changed(1, 2);
		require(key.get() == KeyPanel::State::SHOWN, "while uses come");
		key.skill_changed(2, 0);
		require(key.get() == KeyPanel::State::CLOSING, "and closes after the last");
		key.finished();
		require(key.get() == KeyPanel::State::HIDDEN, "then hides");

		require(clock_text(125) == std::array<std::string, 2>{ "02", "05" }, "minutes and seconds, two digits");
		require(clock_text(-3) == std::array<std::string, 2>{ "00", "00" }, "never below 00:00");

		require(percent(25, 25, 25, 50) == 25, "a share of all three");
		require(percent(1, 1, 0, 0) == 100 && percent(0, 0, 0, 0) == 0, "of 1 with none");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << std::endl;
		return 1;
	}

	std::cout << "massacre tests passed" << std::endl;
	return 0;
}
