#include "../Character/Pets.h"
#include "../Net/Handlers/Helpers/PetParser.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
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

	Writer& string(const std::string& text)
	{
		put<int16_t>(static_cast<int16_t>(text.size()));
		bytes.insert(bytes.end(), text.begin(), text.end());
		return *this;
	}

	// A pet as the server writes it (write_pet).
	Writer& pet(int32_t itemid, const std::string& name, int64_t serial, int16_t x, int16_t y, uint8_t stance, int16_t fh)
	{
		put(itemid).string(name).put(serial).put(x).put(y).put(stance).put(fh);
		return put<int8_t>(0).put<int8_t>(0);
	}
};

void compacting_list()
{
	using ms::pets::insert;
	using ms::pets::remove;

	std::array<int, 3> slots = {};
	insert(slots, 0, 1);
	insert(slots, 1, 2);
	require(slots == std::array<int, 3>{ 1, 2, 0 }, "pets join at their index");

	insert(slots, 0, 3);
	require(slots == std::array<int, 3>{ 3, 1, 2 }, "a pet put first moves the rest up");

	insert(slots, 1, 4);
	require(slots == std::array<int, 3>{ 3, 4, 1 }, "the last falls off a full list");

	remove(slots, 0);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "the rest close up when one leaves");

	remove(slots, 2);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "an empty slot leaves nothing to close");

	insert(slots, 3, 5);
	remove(slots, 3);
	require(slots == std::array<int, 3>{ 4, 1, 0 }, "a fourth index is ignored");
}

void summoning()
{
	using ms::pets::Summon;
	using ms::pets::summon;

	require(summon(false, { 7, 0, 0 }, 8) == Summon::SEND, "without Follow the Lead the server decides");
	require(summon(true, { 0, 0, 0 }, 8) == Summon::SEND, "the first pet comes out");
	require(summon(true, { 7, 0, 0 }, 8) == Summon::ASK_BOSS, "a second asks whether it leads");
	require(summon(true, { 7, 9, 0 }, 9) == Summon::SEND, "a pet that is out goes back");
	require(summon(true, { 7, 9, 6 }, 8) == Summon::FULL, "a fourth is refused");
	require(summon(true, { 7, 9, 6 }, 6) == Summon::SEND, "even with three out, one goes back");

	require(ms::pets::follow_the_lead(0) == 8 && ms::pets::follow_the_lead(412) == 8, "an Explorer's Follow the Lead");
	require(ms::pets::follow_the_lead(1000) == 10000018, "a Noblesse's");
	require(ms::pets::follow_the_lead(2112) == 20000024, "a Legend's");
}

void stances()
{
	using ms::pets::shown;

	require(shown(2) == 0 && shown(3) == 0, "1 moves, either way");
	require(shown(4) == 1 && shown(5) == 1, "2 stands");
	require(shown(6) == 3, "3 jumps");
	require(shown(12) == 4, "6 flies");
	require(shown(22) == 2 && shown(24) == 5, "11 stands otherwise, 12 is hungry");
	require(shown(26) == 6 && shown(28) == 7 && shown(30) == 8, "13 and 14 rest, 15 hangs");
	require(shown(0) == 0 && shown(40) == 0, "the rest move");
}

void spawn_player_pets()
{
	Writer w;
	w.put<int8_t>(1).pet(5000066, "Baby Tiger", 77, 100, -20, 4, 12);
	w.put<int8_t>(1).pet(5000000, "Brown Kitty", 78, 90, -20, 5, 12);
	w.put<int8_t>(1).pet(5000017, "Robo", 79, 80, -20, 2, 12);
	w.put<int8_t>(0).put<int32_t>(0x11223344);

	ms::InPacket recv(w.bytes.data(), w.bytes.size());
	std::vector<ms::PetEntry> pets = ms::PetParser::parse_pets(recv);

	require(pets.size() == 3, "three pets out read as three");
	require(pets[0].itemid == 5000066 && pets[0].name == "Baby Tiger" && pets[0].serial == 77, "the first pet");
	require(pets[0].position == ms::Point<int16_t>(100, -20) && pets[0].stance == 4 && pets[0].fh == 12,
			"where it stands");
	require(pets[2].name == "Robo" && pets[2].serial == 79, "the third pet");
	require(recv.read_int() == 0x11223344, "the 0 after the third ends the list");

	Writer none;
	none.put<int8_t>(0).put<int32_t>(5);
	ms::InPacket empty(none.bytes.data(), none.bytes.size());
	require(ms::PetParser::parse_pets(empty).empty() && empty.read_int() == 5, "no pets is a lone 0");
}

void words()
{
	using ms::pets::split_words;
	using Words = std::vector<std::string>;

	require(split_words("Sleep|Sleepy|Doze|Tired") == Words{ "Sleep", "Sleepy", "Doze", "Tired" }, "split on bars");
	require(split_words(" Hungry? | Food? ") == Words{ "Hungry? ", "Food?" },
			"trimmed in front, and the last at the back too");
	require(split_words("Poo") == Words{ "Poo" }, "a single word");
	require(split_words("a||b") == Words{ "a", "", "b" }, "an empty word is kept between bars");
	require(split_words("  ").empty() && split_words("a|").size() == 1, "nothing after the last bar adds nothing");
	require(ms::pets::trim("\t Tom! \r\n") == "Tom!", "trim strips spaces, tabs and line breaks");
}

void hearing()
{
	using ms::pets::Command;
	using ms::pets::hear;

	std::vector<Command> commands = {
		{ 1, 9, { "Hungry?", "Food?" }, {}, {} },
		{ 10, 30, { "Hungry?", "Food?" }, {}, {} },
		{ 1, 30, { "Sleep", "Doze" }, {}, {} },
	};

	auto heard = hear("  Sleep ", "Baby Tiger", "Baby Tiger", 1, commands);
	require(heard && heard->command == 2 && !heard->named, "a word anywhere in the list, trimmed");
	require(!hear("sleep", "Baby Tiger", "Baby Tiger", 1, commands), "case counts");
	require(!hear("Sleep now", "Baby Tiger", "Baby Tiger", 1, commands), "the whole line must be the word");

	heard = hear("Food?", "Baby Tiger", "Baby Tiger", 12, commands);
	require(heard && heard->command == 1, "the entry for the pet's level");
	require(!hear("Food?", "Baby Tiger", "Baby Tiger", 31, commands), "no entry past its levels");

	heard = hear("Tom! ,~ Doze", "Tom", "Baby Tiger", 5, commands);
	require(heard && heard->command == 2 && heard->named, "a renamed pet called by name");
	heard = hear("TomDoze", "Tom", "Baby Tiger", 5, commands);
	require(heard && heard->named, "no space needed after the name");
	require(!hear("Baby Tiger Doze", "Baby Tiger", "Baby Tiger", 5, commands), "a kind's own name doesn't call it");
	require(!hear("Tom", "Tom", "Baby Tiger", 5, commands), "the name alone isn't a command");
}

void actions()
{
	using ms::pets::action;
	using ms::pets::action_name;
	using ms::pets::reserved;

	std::vector<std::string> customs = { "angry", "chat", "fly", "love" };

	require(action("move", customs) == 0 && action("rest0", customs) == 6 && action("hang", customs) == 8,
			"the base actions");
	require(action("fly", customs) == 4, "a base name wins over the pet's own");
	require(action("angry", customs) == 9 && action("love", customs) == 12, "the pet's own follow");
	require(action("say", customs) == -1 && action("stand", customs) == -1, "one it lacks is -1");
	require(action_name(6, customs) == "rest0" && action_name(10, customs) == "chat", "and back");
	require(action_name(-1, customs).empty() && action_name(13, customs).empty(), "none for none");

	require(reserved("info") && reserved("Interact") && reserved("stand1") && reserved("autoSpeaking"),
			"the image's other children");
	require(!reserved("move") && !reserved("fly") && !reserved("love"), "animations are not");

	require(!ms::pets::idle_due(4992, 8) && ms::pets::idle_due(5000, 8) && !ms::pets::idle_due(5008, 8),
			"once at 5 s");
	require(ms::pets::idle_due(10000, 8), "and every 5 s after");
}

void food()
{
	using ms::pets::closeness_change;
	using ms::pets::Command;
	using ms::pets::food_for;
	using ms::pets::hungriest;

	require(ms::pets::is_food(2120000) && !ms::pets::is_food(2000000) && !ms::pets::is_food(5240000),
			"pet food is 212xxxx");
	require(ms::pets::is_cash_food(5240027) && !ms::pets::is_cash_food(2120000) && !ms::pets::is_cash_food(5000066),
			"cash pet food is 524xxxx");

	require(hungriest({ 60, 40, 80 }) == 1, "the least full eats");
	require(hungriest({ 40, 80, 40 }) == 0, "the first of equals");
	require(hungriest({ -1, 90, -1 }) == 1, "an empty slot is skipped");
	require(hungriest({ 100, -1, -1 }) == 0, "a full pet is still the one fed");
	require(hungriest({ -1, -1, -1 }) == -1, "no pet out, none");

	std::vector<Command> table = {
		{ 1, 9, {}, { { "eat", { "Chomp" } } }, { { "cry", {} } } },
		{ 10, 19, {}, {}, {} },
		{ 1, 30, {}, {}, {} },
	};

	require(food_for(table, 5) == &table[0] && food_for(table, 9) == &table[0], "the entry for its level");
	require(food_for(table, 12) == &table[1], "the next levels' entry");
	require(food_for(table, 25) == &table[2], "the first that holds it");
	require(!food_for(table, 31) && !food_for(table, -1), "none past its levels, or for another's pet");

	require(closeness_change(3, 4) == "The closeness increased (+1)", "closer");
	require(closeness_change(4, 3) == "The closeness decreased (-1)", "further");
	require(closeness_change(4, 4).empty(), "no change, no line");
}

int main()
{
	try
	{
		compacting_list();
		summoning();
		stances();
		spawn_player_pets();
		words();
		hearing();
		actions();
		food();

		std::cout << "Pets passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
