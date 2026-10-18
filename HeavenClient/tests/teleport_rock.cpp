#include "../Character/TeleportRock.h"
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
		using ms::TeleportRock;

		require(!TeleportRock::is_vip(5040000) && !TeleportRock::is_vip(5040001), "5040xxx rocks are regular");
		require(TeleportRock::is_vip(5041000), "5041000 is the VIP rock");
		require(TeleportRock::slots(false) == 5 && TeleportRock::slots(true) == 10, "five and ten slots");

		require(TeleportRock::may_keep(100000000), "Henesys can be kept");
		require(!TeleportRock::may_keep(10000), "Maple Island can't be kept");
		require(!TeleportRock::may_keep(109000000), "an event map can't be kept");

		require(TeleportRock::trim(" \tNeeerp\r\n") == "Neeerp", "a name is trimmed");
		require(TeleportRock::trim("  ").empty(), "a blank name is empty");

		TeleportRock rock;
		const int32_t E = TeleportRock::EMPTY;

		require(rock.get_maps(false) == std::vector<int32_t>(5, E), "the regular list starts empty");
		require(rock.get_maps(true) == std::vector<int32_t>(10, E), "the VIP list starts empty");

		rock.set_maps(false, { 100000000, E, 104000000 });
		require(rock.get_maps(false) == std::vector<int32_t>{ 100000000, E, 104000000, E, E }, "a short list is padded");
		require(TeleportRock::used(rock.get_maps(false)) == 2, "empty slots aren't used");

		rock.set_maps(false, { 1, 2, 3, 4, 5, 6 });
		require(rock.get_maps(false) == std::vector<int32_t>{ 1, 2, 3, 4, 5 }, "a long list is cut");
		require(rock.get_maps(true) == std::vector<int32_t>(10, E), "the other list is left alone");

		std::vector<int32_t> maps = { 100000000, E, E, E, E };
		require(TeleportRock::keep_problem(maps, 104000000).empty(), "a new map can be kept");
		require(TeleportRock::keep_problem(maps, 100000000) == "You have already entered this map.",
				"a listed map isn't kept twice");
		require(TeleportRock::keep_problem(maps, 10000) == TeleportRock::NOT_FOR_LIST, "Maple Island isn't kept");
		require(TeleportRock::keep_problem({ 1, 2, 3, 4, 5 }, 2).find("is full") != std::string::npos,
				"a full list is checked first");

		require(TeleportRock::target_problem("Friend", "Neeerp").empty(), "another character is a target");
		require(TeleportRock::target_problem("neeERP", "Neeerp").find("yourself") != std::string::npos,
				"the player isn't, in any case");
		require(TeleportRock::target_problem("abc", "Neeerp") == "- The character name is invalid.",
				"a short name is invalid");
		require(TeleportRock::target_problem("abc", "abc") == "- The character name is invalid.",
				"validity is checked first");

		require(TeleportRock::refusal(6, "Friend").rfind("Friend is currently", 0) == 0, "a lost target is named");
		require(TeleportRock::refusal(9, "") == TeleportRock::SAME_MAP, "the same map");
		require(TeleportRock::refusal(5, "") == TeleportRock::refusal(8, ""), "5 and 8 say the same");
		require(TeleportRock::refusal(2, "").empty() && TeleportRock::refusal(3, "").empty(), "a list has no notice");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
