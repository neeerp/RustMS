#include "../Util/NpcTextMarkup.h"
#include <iostream>
#include <utility>
#include <vector>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		ms::NpcTextMarkup::Lookups lookups {
			[]() { return std::string("Applebee"); },
			[](int32_t id) { return id == 2000 ? std::string("Roger") : std::string("?"); },
			[](int32_t id) { return id == 2010000 ? std::string("Apple") : id == 2010009 ? std::string("Green Apple") : std::string("?"); },
			[](int32_t id) { return id == 100100 ? std::string("Snail") : std::string("?"); },
			[](int32_t id) { return id == 10000 ? std::string("Mushroom Town") : std::string("?"); }
		};
		auto expand = [&](const std::string& s) { return ms::NpcTextMarkup::expand(s, lookups); };

		require(expand("#fUI/UIWindow.img/QuestIcon/4/0#\\n#v2010000# 3 #t2010000#\\n#v2010009# 3 #t2010009#") ==
			"\\n 3 Apple\\n 3 Green Apple", "every item tag expands and images are dropped");
		require(expand("Hi #h0#, I'm #p2000#.") == "Hi Applebee, I'm Roger.", "player and NPC names");
		require(expand("Hunt #o100100# in #m10000#, #z2010000#.") == "Hunt Snail in Mushroom Town, Apple.", "mob, map, item names");
		require(expand("#bBlue#k and #L0#a choice#l") == "#bBlue#k and #L0#a choice#l", "colours and choices are left alone");
		require(expand("a # sign") == "a # sign", "a lone # survives");
		using Choices = std::vector<std::pair<int32_t, std::string>>;
		auto choices = ms::NpcTextMarkup::choices;
		require(choices("Pick:\\r\\n#L0#One#l\\r\\n#L1#Two#l") == Choices{{0, "One"}, {1, "Two"}}, "closed choices");
		// Say.img/1036/1/2: the last two choices have no #l.
		require(choices("#b#L0# Warrior  -  Perion #l\\r\\n#L1# Magician  -  Ellinia #l \\r\\n#b#L2# Bowman  -  Henesys \\r\\n#b#L3# Thief  -  Nautilus") ==
			Choices{{0, " Warrior  -  Perion"}, {1, " Magician  -  Ellinia"}, {2, " Bowman  -  Henesys"}, {3, " Thief  -  Nautilus"}},
			"unclosed choices end at the next #L or the end");
		require(choices("no menu here").empty(), "text without choices");
		require(choices("#Lx# broken").empty(), "a non-numeric id is not a choice");
		std::cout << "NPC text markup passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
