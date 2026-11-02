#include "../Character/QuestAlarm.h"
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
		using namespace ms::QuestAlarm;
		using Ids = std::vector<int16_t>;

		require(key(0, 4) == "0_4", "world and character");
		require(find("", "0_4") == Saved(), "nothing saved: empty, AUTO on");
		require(find("0_4:0:2000,1001", "0_4") == Saved({ Ids({ 2000, 1001 }), false }), "the saved order and AUTO");
		require(find("0_4:0:2000,1001", "0_5") == Saved(), "another character's list is theirs");
		require(find("0_4:1:", "0_4") == Saved({ Ids(), true }), "an empty list");
		require(find("0_4:1:7,x,,7,-3,99999,8", "0_4").quests == Ids({ 7, 8 }), "bad ids and repeats dropped");
		require(find("0_4:1:1,2,3,4,5,6", "0_4").quests.size() == MAX, "five at most");
		require(find("junk;:1:5;0_4:1:9", "0_4").quests == Ids({ 9 }), "bad entries skipped");

		std::string all = store("", "0_4", { Ids({ 2000, 1001 }), true });
		all = store(all, "0_7", { Ids(), false });
		require(all == "0_4:1:2000,1001;0_7:0:", "every character in one value");
		require(find(store(all, "0_4", { Ids({ 1001 }), true }), "0_4").quests == Ids({ 1001 }), "saving replaces");
		require(find(store(all, "0_4", { Ids({ 1001 }), true }), "0_7") == Saved({ Ids(), false }), "and keeps the others");

		std::map<int16_t, int64_t> automatic = { { 30, 0 }, { 40, 0 } };
		require(manual_slot({ 10, 30, 20, 40 }, automatic) == 1, "by hand goes above the first automatic one");
		require(manual_slot({ 10, 20 }, automatic) == 2, "or at the end");
		require(manual_slot({}, automatic) == 0, "or first");
	}
	catch (const std::exception& e)
	{
		std::cerr << "quest_alarm: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "quest_alarm: ok" << std::endl;
	return 0;
}
