#include "../Gameplay/MapleMap/ObjStateRules.h"
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
		using namespace ms::obj_state;
		using End = ms::Animation::End;

		Playback gate = playback(2);
		require(gate.animates && gate.end == End::LOOP && gate.loop_from == 2 && !gate.waits,
			"KPQ's gate fades in once, then loops from frame 2");
		Playback plain = playback(0);
		require(plain.end == End::LOOP && plain.loop_from == 0, "no repeat loops the lot");

		Playback mirror = playback(-1);
		require(mirror.animates && mirror.end == End::HOLD && !mirror.waits, "-1 plays once and stays");

		Playback spear = playback(-2);
		require(spear.animates && spear.end == End::FIRST && spear.waits, "-2 plays once, back to the start");

		require(!playback(-3).animates, "below -2 nothing animates");

		require(next_state(1, 4, -1) == 1, "-1 shows the current state again");
		require(next_state(0, 4, 3) == 3 && next_state(2, 4, 2) == 2, "a state it has");
		require(next_state(0, 4, 4) == -1 && next_state(0, 1, 1) == -1, "one it hasn't changes nothing");
		require(next_state(0, 4, -2) == -1, "nor a negative one");

		using Tags = std::vector<std::string>;
		require(split_tags("bowQuest ") == Tags{ "bowQuest" }, "the spaces around a tag go");
		require(split_tags("james1;james2;james3;") == Tags{ "james1;james2;james3;" }, "\";\" doesn't split");
		require(split_tags(" a , b,c") == Tags{ "a", "b", "c" }, "\",\" does");
		require(split_tags("a,, ,b,") == Tags{ "a", "b" }, "empty ones go");
		require(split_tags("").empty(), "no tags");

		std::cout << "Obj state rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
