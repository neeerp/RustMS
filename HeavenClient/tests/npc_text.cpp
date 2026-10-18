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
		using Images = std::vector<ms::NpcTextMarkup::InlineImage>;

		require(expand("#fUI/UIWindow.img/QuestIcon/4/0#\\n#v2010000# 3 #t2010000#\\n#v2010009# 3 #t2010009#") ==
			"\\n 3 Apple\\n 3 Green Apple", "every item tag expands and images are dropped");
		require(expand("Hi #h0#, I'm #p2000#.") == "Hi Applebee, I'm Roger.", "player and NPC names");
		require(expand("Hunt #o100100# in #m10000#, #z2010000#.") == "Hunt Snail in Mushroom Town, Apple.", "mob, map, item names");
		require(expand("#bBlue#k and #L0#a choice#l") == "#bBlue#k and #L0#a choice#l", "colours and choices are left alone");
		require(expand("a # sign") == "a # sign", "a lone # survives");
		Images reward;
		require(ms::NpcTextMarkup::expand("#v2010000:# #t2010000:# 3 #z2010009:#", lookups, &reward) == "\x01 Apple 3 Green Apple",
			"ids may end in a colon");
		require(reward == Images{{'v', "2010000"}}, "the colon is not part of the id");
		Images images;
		require(ms::NpcTextMarkup::expand("#fUI/UIWindow.img/QuestIcon/4/0#\\n#v2010000# 3 #t2010000#\\n#s1001# and #i2010009##fnot an image#",
			lookups, &images) == "\x01\\n\x01 3 Apple\\n\x01 and \x01#fnot an image#", "captured images leave placeholders");
		require(images == Images{{'f', "UI/UIWindow.img/QuestIcon/4/0"}, {'v', "2010000"}, {'s', "1001"}, {'i', "2010009"}},
			"captured images in order");
		using Choices = std::vector<std::pair<int32_t, std::string>>;
		auto choices = ms::NpcTextMarkup::choices;
		require(choices("Pick:\\r\\n#L0#One#l\\r\\n#L1#Two#l") == Choices{{0, "One"}, {1, "Two"}}, "closed choices");
		// Say.img/1036/1/2: the last two choices have no #l.
		require(choices("#b#L0# Warrior  -  Perion #l\\r\\n#L1# Magician  -  Ellinia #l \\r\\n#b#L2# Bowman  -  Henesys \\r\\n#b#L3# Thief  -  Nautilus") ==
			Choices{{0, " Warrior  -  Perion"}, {1, " Magician  -  Ellinia"}, {2, " Bowman  -  Henesys"}, {3, " Thief  -  Nautilus"}},
			"unclosed choices end at the next #L or the end");
		require(choices("no menu here").empty(), "text without choices");
		require(choices("#Lx# broken").empty(), "a non-numeric id is not a choice");
		std::string menu = "#d#L1# Pio#l#k\r\n#d#L2# #r#e[Job Adv.]#d#nGrendel#l#k\r\n";
		require(ms::NpcTextMarkup::colour_at(menu, menu.find("#L1")) == "#d", "a choice takes the colour before it");
		require(ms::NpcTextMarkup::colour_at(menu, menu.find("#L2")) == "#d", "after the last one's #k");
		require(ms::NpcTextMarkup::colour_at("#L1#bananas#l#L2#", 12) == "", "a choice's id is not a colour");
		auto mark = [](const std::string& s, std::vector<int32_t> expected)
		{
			std::vector<int32_t> ids;
			std::string marked = ms::NpcTextMarkup::mark_choices(s, ids);
			require(ids == expected, "marked choice ids");
			return marked;
		};
		require(mark("Pick:\\r\\n#L0#One#l\\r\\n#L1#Two#l", { 0, 1 }) == "Pick:\\r\\n\x02One\x03\\r\\n\x02Two\x03",
			"closed choices are marked and lose their #l");
		require(mark("#L0##bLevel 10#l#l\\n#L1#Level 8#k#l", { 0, 1 }) == "\x02#bLevel 10\x03#l\\n\x02Level 8\x03#k",
			"a trailing colour code stays after the end marker");
		require(mark("#b#L2# Bowman  -  Henesys \\r\\n#b#L3# Thief", { 2, 3 }) == "#b\x02 Bowman  -  Henesys\x03 \\r\\n#b\x02 Thief\x03",
			"unclosed choices end at the next #L or the end");
		require(mark("no menu here", {}) == "no menu here", "text without choices is unchanged");
		std::cout << "NPC text markup passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
